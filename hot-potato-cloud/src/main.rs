use std::net::{TcpListener, TcpStream, SocketAddr, Shutdown};
use std::io::{prelude::*, BufReader};
use std::time::{Duration, SystemTime};

const ADDR: &'static str = "127.0.0.1:27491";
const MAX_REPLICATION: u8 = 1;


struct File {
    data: String,
    last_seen: SystemTime,
    replicas_sent: u8,
}


fn main() {
    let listener = TcpListener::bind(ADDR).unwrap();
    let mut peers: Vec<SocketAddr> = Vec::new();
    let mut files: Vec<File> = Vec::new();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let mut req_type = [0;1];
        let _ = stream.peek(&mut req_type).unwrap();
        match req_type[0] {
            b'p' => accept_new_peer(stream, &mut peers),
            b'd' => accept_data(stream, &mut files),
            _ => stream.shutdown(Shutdown::Both).unwrap(),
        };
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

fn accept_data(mut stream: TcpStream, files: &mut Vec<File>){
    let mut br = BufReader::new(&mut stream);
    let mut f = File{
        data: String::new(),
        last_seen: SystemTime::now(),
        replicas_sent: 0,
    };

    let x = br.read_line(&mut f.data).unwrap();
    f.data.remove(0); // First char is reserved for protocol.

    if x < 1 { // File empty
        stream.write(b"ERR\n").unwrap();
    }

    for known in files.iter_mut() {
        if known.data == f.data { // File known
            known.last_seen = SystemTime::now();
            stream.write(b"KNOWN\n").unwrap();
            stream.shutdown(Shutdown::Both).unwrap();
            return ()
        }
    }
    stream.write(b"OK\n").unwrap();
    files.push(f); // File new
    stream.shutdown(Shutdown::Both).unwrap();
    return ()
}