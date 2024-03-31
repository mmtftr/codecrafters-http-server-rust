use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use nom::branch::alt;
use nom::bytes::streaming::{tag_no_case as tag, take_until};
use nom::IResult;
use thiserror::Error;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    if let Err(err) = listen_to_incoming(listener) {
        println!("Error while listening: {:?}", err);
    }
}

fn listen_to_incoming(listener: TcpListener) -> Result<(), std::io::Error> {
    for stream in listener.incoming() {
        let _ = handle_connection(stream?);
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

fn send_text_content(stream: &mut TcpStream, txt: &str) -> Result<(), std::io::Error> {
    stream.write("HTTP/1.1 200 OK\r\n".as_bytes())?;
    stream.write("Content-Type: text/plain\r\n".as_bytes())?;
    stream.write(format!("Content-Length: {}\r\n", txt.len()).as_bytes())?;
    stream.write("\r\n".as_bytes())?;
    stream.write(txt.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), ConnectionError> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?; // read 1K bytes for now

    let utf8 = String::from_utf8_lossy(&buffer[..]);
    let req = parse_request(&utf8)
        .map_err(|e| ConnectionError::ParsingError(e.to_string()))?
        .1;

    if req.path == "/" {
        stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
    } else if req.path.starts_with("/echo/") {
        let txt = &req.path[6..];
        send_text_content(&mut stream, txt).unwrap();
    } else {
        stream
            .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
            .unwrap();
    }

    println!("Request: {req:?}");

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
    // todo!
    Ok(("", vec![]))
}

fn parse_request(input: &str) -> IResult<&str, Request> {
    let (rest, method) = alt((tag("GET"), tag("POST")))(input)?;
    let (rest, _) = tag(" ")(rest)?;
    let (rest, path) = take_until(" ")(rest)?;
    let (rest, _) = tag(" ")(rest)?;
    let (rest, version) = take_until("\r\n")(rest)?;

    let (rest, headers) = parse_headers(input)?;

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
