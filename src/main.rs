use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net;
use std::result::Result;
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
struct Message<T> {
    msg: T,
    from: net::SocketAddr,
}

impl<T> fmt::Display for Message<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.from, self.msg)
    }
}

struct Client<T> {
    tx: mpsc::SyncSender<T>,
}

impl<T> Client<T>
where
    T: 'static + fmt::Display + Send + Sync,
{
    fn new(tx: mpsc::SyncSender<T>) -> Self {
        Client { tx }
    }
}

impl<T> Client<Message<T>>
where
    T: 'static + Send + Sync + From<String>,
{
    fn handle(self: Self, stream: net::TcpStream) -> Result<(), Box<Error>> {
        let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
        let tx = self.tx.clone();
        println!("{} -> {}", peer, local);
        thread::spawn(move || -> Result<(), io::Error> {
            for line in BufReader::new(stream).lines() {
                tx.send(Message {
                    msg: From::from(line?),
                    from: peer,
                })
                .or_else(|e| {
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

struct Server<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> Server<T>
where
    T: 'static + fmt::Display + Send,
{
    fn new(rx: mpsc::Receiver<T>) -> Self {
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
    let (tx, rx) = mpsc::sync_channel::<Message<String>>(1000);
    let _ = Server::new(rx).start();
    let listener = net::TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => Client::new(tx.clone()).handle(stream),
            Err(e) => Err(Box::from(e)),
        }?;
    }
    Ok(())
}
