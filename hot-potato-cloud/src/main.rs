use std::net::{TcpListener, TcpStream, SocketAddr, Shutdown};
use std::io::prelude::*;

const ADDR: &'static str = "127.0.0.1:27491";

fn main() {
    let listener = TcpListener::bind(ADDR).unwrap();
    let mut peers: Vec<SocketAddr> = Vec::new();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        accept_new_peer(stream, &mut peers)
    }
}

fn accept_new_peer(mut stream: TcpStream, peers: &mut Vec<SocketAddr>){
    let peer = stream.peer_addr().unwrap();

    for x in peers.iter() {
        stream.write(format!("{}:{} ", x.ip(), x.port()).as_bytes()).unwrap();
    }

    stream.shutdown(Shutdown::Both).unwrap();

    let mut known = false;
    for other in peers.iter() {
        if peer.ip() == other.ip() && peer.port() == other.port() {
            known = true;
            break;
        }
    }
    if !known {
        peers.push(peer);
    }

    return ()
}