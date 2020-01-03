use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net;
use std::thread;

struct Client {
    stream: net::TcpStream,
}

impl Client {
    fn new(stream: net::TcpStream) -> Self {
        Client { stream }
    }

    fn handle(self: Self) -> io::Result<()> {
        let stream = self.stream;
        let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
        println!("{} -> {}", peer, local);
        thread::spawn(move || -> io::Result<()> {
            for line in BufReader::new(stream).lines() {
                println!("{} {}", peer, line?);
            }
            println!("{} ~> {}", peer, local);
            Ok(())
        });
        Ok(())
    }
}

fn main() -> io::Result<()> {
    println!("chat server");
    let listener = net::TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => Client::new(stream).handle(),
            Err(e) => Err(e),
        }?;
    }
    Ok(())
}
