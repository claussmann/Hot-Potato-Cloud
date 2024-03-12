use std::net::{TcpListener, TcpStream, Shutdown, IpAddr};
use std::io::{prelude::*, BufReader};
use std::time::{Duration, SystemTime};

const ADDR: &'static str = "127.0.0.1:27491";
const MAX_REPLICATION: u8 = 1;


struct File {
    data: String,
    last_seen: SystemTime,
    replicas_sent: u8,
}

struct Peer {
    addr: IpAddr,
    last_seen: SystemTime,
}


fn main(){
    let listener = TcpListener::bind(ADDR).unwrap();
    let mut peers: Vec<Peer> = Vec::new();
    let mut files: Vec<File> = Vec::new();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        register_peer_addr(&stream, &mut peers);
        let mut req_type = [0;1];
        let _ = stream.peek(&mut req_type).unwrap();
        match req_type[0] {
            b'p' => welcome_new_peer(stream, &peers),
            b'd' => receive_data(stream, &mut files),
            _ => stream.shutdown(Shutdown::Both).unwrap(),
        };
    }
}

fn register_peer_addr(stream: &TcpStream, peers: &mut Vec<Peer>){
    let peer_addr = stream.peer_addr().unwrap().ip();
    for other in peers.iter_mut() {
        if peer_addr == other.addr {
            other.last_seen = SystemTime::now();
            return ();
        }
    }
    let peer = Peer {
        addr: peer_addr,
        last_seen: SystemTime::now(),
    };
    peers.push(peer);
    return ()
}

fn welcome_new_peer(mut stream: TcpStream, peers: &Vec<Peer>){
    for x in peers.iter() {
        stream.write(format!("{} ", x.addr).as_bytes()).unwrap();
    }
    stream.write("\n".as_bytes()).unwrap();
    stream.shutdown(Shutdown::Both).unwrap();
    return ()
}

fn receive_data(mut stream: TcpStream, files: &mut Vec<File>){
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