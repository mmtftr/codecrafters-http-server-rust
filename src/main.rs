use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    if let Err(err) = listen_to_incoming(listener) {
        println!("Error while listening: {:?}", err);
    }
}

fn listen_to_incoming(listener: TcpListener) -> Result<(), std::io::Error> {
    for stream in listener.incoming() {
        handle_connection(stream?);
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap(); // read 1K bytes for now

    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();

    println!("Request: {}", String::from_utf8_lossy(&buffer[..]));
}
