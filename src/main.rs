use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::net;
use std::result::Result;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
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

struct Server {
    streams: Arc<Mutex<Vec<net::TcpStream>>>,
}

impl Server {
    fn new() -> Self {
        Server {
            streams: Arc::new(Mutex::new(vec![])),
        }
    }

    fn start<T>(&self, rx: mpsc::Receiver<Message<T>>)
    where
        T: 'static + Sync + Send,
        T: fmt::Display,
    {
        let streams = self.streams.clone();
        thread::spawn(
            move || -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
                for msg in rx.iter() {
                    println!("{}", msg);
                    match streams.lock() {
                        Ok(mut streams) => {
                            streams.retain(|stream| Server::send_peers(stream, &msg).is_ok())
                        }
                        Err(e) => println!("{}", e),
                    }
                }
                Ok(())
            },
        );
    }

    fn handle<T>(
        self: &Self,
        tx: mpsc::SyncSender<Message<T>>,
        stream: net::TcpStream,
    ) -> Result<(), Box<dyn Error>>
    where
        T: fmt::Display + From<String>,
        T: 'static + Send + Sync,
    {
        let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
        println!("{} -> {}", peer, local);
        let buf_reader = io::BufReader::new(stream.try_clone()?);
        thread::spawn(move || -> Result<(), io::Error> {
            for line in buf_reader.lines() {
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
        let streams = self.streams.clone();
        let mut streams = streams.lock().unwrap(); // TODO
        streams.push(stream);
        Ok(())
    }

    fn send_peers<T>(mut stream: &net::TcpStream, msg: &Message<T>) -> Result<(), impl Error>
    where
        T: fmt::Display,
    {
        let peer = stream.peer_addr()?;
        if msg.from == peer {
            Ok(())
        } else {
            let buf = format!("{}\n", msg);
            let buf = buf.as_bytes();
            io::BufWriter::new(stream)
                .write_all(buf)
                .and(stream.flush())
                .and(Ok(()))
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("chat server");
    let (server_tx, server_rx) = mpsc::sync_channel::<Message<String>>(1000);
    let mut server = Server::new();
    server.start(server_rx);
    let listener = net::TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = &mut server;
                server.handle(server_tx.clone(), stream)
            }
            Err(e) => Err(Box::from(e)),
        }?;
    }
    Ok(())
}
