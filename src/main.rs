use async_std::{
    prelude::*,
    task,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};
use futures::channel::mpsc;
use futures::SinkExt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type StreamRes = std::result::Result<TcpStream, std::io::Error>;

async fn accept_on_addrs(addrs: impl ToSocketAddrs) -> Result<impl Stream<Item = StreamRes>> {
    let (tx, rx) = mpsc::unbounded();
    for addr in addrs.to_socket_addrs().await? {
        let listener = TcpListener::bind(addr).await?;
        accept_connections(listener, tx.clone());
    }
    Ok(rx)
}

fn accept_connections(listener: TcpListener, mut tx: Sender<StreamRes>) {
    task::spawn(async move {
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            tx.send(stream).await.unwrap();
        }
    });
}

async fn accept_loop(addrs: impl ToSocketAddrs) -> Result<()> {
    let mut count: usize = 0;
    let mut incoming = accept_on_addrs(addrs).await?;
    while let Some(stream) = incoming.next().await {
        count += 1;
        spawn_and_log_error(handle_client(stream, count));
    }
    Ok(())
}

async fn handle_client(stream: StreamRes, count: usize) -> Result<()> {
    let mut stream = stream?;
    println!("Accepting from: {}", stream.peer_addr()?);
    let msg = format!("{}\n", count);
    stream.write_all(msg.as_bytes()).await?;
    Ok(())
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{}", e)
        }
    })
}

fn main() -> Result<()> {
    let addrs: [SocketAddr; 2] = [
        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8081),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8081),
    ];
    let fut = accept_loop(&addrs[..]);
    task::block_on(fut)
}
