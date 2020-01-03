use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(stream: TcpStream) -> io::Result<()> {
    let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
    println!("{} -> {}", peer, local);
    thread::spawn(move || -> io::Result<()> {
        for line in BufReader::new(stream).lines() {
            println!("{}", line?);
        }
        println!("{} ~> {}", peer, local);
        Ok(())
    });
    Ok(())
}

fn main() -> io::Result<()> {
    println!("Hello, world!");
    let listener = TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => Err(e),
        }?;
    }
    Ok(())
}
