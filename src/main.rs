use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net;
use std::result::Result;
use std::sync::mpsc;
use std::thread;

struct Client {
    stream: net::TcpStream,
    tx: mpsc::SyncSender<String>,
}

impl Client {
    fn new(stream: net::TcpStream, tx: mpsc::SyncSender<String>) -> Self {
        Client { stream, tx }
    }

    fn handle(self: Self) -> Result<(), Box<Error>> {
        let stream = self.stream;
        let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
        let tx = self.tx.clone();
        println!("{} -> {}", peer, local);
        thread::spawn(move || -> Result<(), io::Error> {
            for line in BufReader::new(stream).lines() {
                // let line = line?;
                // println!("{} {}", peer, &line);
                tx.send(line?).or_else(|e| {
                    println!("{} {}", peer, e);
                    Err(io::Error::new(io::ErrorKind::Other, e))
                })?
            }
            println!("{} ~> closed", peer);
            Ok(())
        });
        Ok(())
    }
}

struct Server {
    rx: mpsc::Receiver<String>,
}

impl Server {
    fn new(rx: mpsc::Receiver<String>) -> Self {
        Server { rx }
    }

    fn start(self) {
        thread::spawn(move || -> Result<(), mpsc::RecvError> {
            for string in self.rx.iter() {
                println!("{}", string);
            }
            Ok(())
        });
    }
}

fn main() -> Result<(), Box<Error>> {
    println!("chat server");
    let (tx, rx) = mpsc::sync_channel(1000);
    let _ = Server::new(rx).start();
    let listener = net::TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => Client::new(stream, tx.clone()).handle(),
            Err(e) => Err(Box::from(e)),
        }?;
    }
    Ok(())
}
