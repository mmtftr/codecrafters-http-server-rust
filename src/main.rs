use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

use itertools::Itertools;
use nom::branch::alt;
use nom::bytes::streaming::{tag_no_case as tag, take_until};
use nom::IResult;
use thiserror::Error;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let path = std::env::args()
        .tuple_windows()
        .filter_map(|(arg1, arg2)| {
            if arg1 == "--directory" {
                Some(arg2)
            } else {
                None
            }
        })
        .next();
    let path = path.map(|s| PathBuf::from(s));

    if let Err(err) = serve(listener, path) {
        println!("Error while listening: {:?}", err);
    }
}

fn serve(listener: TcpListener, path: Option<PathBuf>) -> Result<(), std::io::Error> {
    for stream in listener.incoming() {
        let path = path.clone();
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream.unwrap(), path) {
                println!("Error while handling connection: {:?}", e)
            }
        });
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Parsing error: {0}")]
    ParsingError(String), // TODO: improve parsing error
}

fn send_content(
    stream: &mut TcpStream,
    content_type: &str,
    cnt: impl AsRef<[u8]>,
) -> Result<(), std::io::Error> {
    stream.write("HTTP/1.1 200 OK\r\n".as_bytes())?;
    stream.write(format!("Content-Type: {content_type}\r\n").as_bytes())?;
    stream.write(format!("Content-Length: {}\r\n", cnt.as_ref().len()).as_bytes())?;
    stream.write("\r\n".as_bytes())?;
    stream.write(cnt.as_ref())?;
    stream.flush()?;
    Ok(())
}

fn send_text_content(stream: &mut TcpStream, txt: &str) -> Result<(), std::io::Error> {
    send_content(stream, "text/plain", txt.as_bytes())
}

fn not_found(stream: &mut TcpStream) -> Result<(), std::io::Error> {
    stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())?;
    Ok(())
}

fn handle_connection(mut stream: TcpStream, path: Option<PathBuf>) -> Result<(), ConnectionError> {
    let mut buffer = [0; 1024];
    let len = stream.read(&mut buffer)?; // read 1K bytes for now

    let utf8 = String::from_utf8_lossy(&buffer[..=len]);
    let req = parse_request(&utf8)
        .map_err(|e| ConnectionError::ParsingError(e.to_string()))?
        .1;

    println!("Request: {req:?}");

    if req.path == "/" {
        stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
    } else if req.path.starts_with("/echo/") {
        let txt = &req.path[6..];
        send_text_content(&mut stream, txt).unwrap();
    } else if req.path.starts_with("/files/") {
        let path = path.unwrap_or_else(|| PathBuf::from("."));
        let file_path = path.join(&req.path[7..]);

        let file = std::fs::read_to_string(file_path)?;
        send_content(&mut stream, "application/octet-stream", file);
    } else if req.path == "/user-agent" {
        let ua = req
            .headers
            .iter()
            .find(|(k, _)| *k == "User-Agent")
            .map(|(_, v)| v)
            .unwrap_or(&"Unknown");

        send_text_content(&mut stream, ua).unwrap();
    } else {
        not_found(&mut stream)?;
    }

    stream.flush()?;

    stream.shutdown(std::net::Shutdown::Write)?;

    Ok(())
}

#[derive(Debug)]
pub struct Request<'a> {
    method: &'a str,
    path: &'a str,
    version: &'a str,
    headers: Vec<(&'a str, &'a str)>,
}

fn parse_headers(input: &str) -> IResult<&str, Vec<(&str, &str)>> {
    let mut headers = vec![];

    let mut rest = input;

    loop {
        let Ok((rst, name)): IResult<&str, &str> = take_until(":")(rest) else {
            break;
        };
        let (rst, _) = tag(": ")(rst)?;
        let (rst, val) = take_until("\r\n")(rst)?;
        let (rst, _) = tag("\r\n")(rst)?;

        rest = rst;

        headers.push((name, val));
    }

    Ok((rest, headers))
}

fn parse_request(input: &str) -> IResult<&str, Request> {
    let (rest, method) = alt((tag("GET"), tag("POST")))(input)?;
    let (rest, _) = tag(" ")(rest)?;
    let (rest, path) = take_until(" ")(rest)?;
    let (rest, _) = tag(" ")(rest)?;
    let (rest, version) = take_until("\r\n")(rest)?;
    let (rest, _) = tag("\r\n")(rest)?;

    let (rest, headers) = parse_headers(rest)?;

    return Ok((
        rest,
        Request {
            method,
            path,
            version,
            headers,
        },
    ));
}
