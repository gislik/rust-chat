use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::net;
use std::result::Result;
use std::sync;
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

struct Server<T> {
    txs: sync::Arc<sync::Mutex<Vec<mpsc::SyncSender<Message<T>>>>>,
}

impl<T> Server<T>
where
    T: 'static + fmt::Display + From<String> + Clone + Send + Sync,
{
    fn new() -> Self {
        Server {
            txs: sync::Arc::new(sync::Mutex::new(vec![])),
        }
    }

    fn add(&mut self, tx: mpsc::SyncSender<Message<T>>) -> Result<(), Box<dyn Error>> {
        let txs = self.txs.clone();
        let mut txs = txs.lock().unwrap(); // TODO
        txs.push(tx);
        Ok(())
    }

    fn start(&self, rx: mpsc::Receiver<Message<T>>) {
        let txs = self.txs.clone();
        thread::spawn(move || -> Result<(), mpsc::SendError<Message<T>>> {
            for msg in rx.iter() {
                println!("{}", msg);
                let mut txs = txs.lock().unwrap(); // TODO
                txs.retain(|tx| {
                    match tx.send(Message {
                        msg: msg.msg.clone(),
                        from: msg.from,
                    }) {
                        Err(_) => false,
                        _ => true,
                    }
                });
            }
            Ok(())
        });
    }

    fn handle(
        self: &Self,
        tx: mpsc::SyncSender<Message<T>>,
        rx: mpsc::Receiver<Message<T>>,
        stream: net::TcpStream,
    ) -> Result<(), Box<Error>> {
        let (peer, local) = (stream.peer_addr()?, stream.local_addr()?);
        println!("{} -> {}", peer, local);
        let mut buf_writer = io::BufWriter::new(stream.try_clone()?);
        let buf_reader = io::BufReader::new(stream);
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
        thread::spawn(move || -> Result<(), io::Error> {
            for msg in rx.iter() {
                write!(buf_writer, "{}\n", msg)?;
                buf_writer.flush()?;
            }
            Ok(())
        });
        Ok(())
    }
}

fn main() -> Result<(), Box<Error>> {
    println!("chat server");
    let (server_tx, server_rx) = mpsc::sync_channel::<Message<String>>(1000);
    let mut server = Server::new();
    server.start(server_rx);
    let listener = net::TcpListener::bind("localhost:1234")?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let (client_tx, client_rx) = mpsc::sync_channel(1000);
                let server = &mut server;
                server.add(client_tx)?;
                server.handle(server_tx.clone(), client_rx, stream)
            }
            Err(e) => Err(Box::from(e)),
        }?;
    }
    Ok(())
}
