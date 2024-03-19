use std::net::{TcpListener, TcpStream, SocketAddr, Shutdown, IpAddr, Ipv4Addr};
use std::io::{prelude::*, BufReader};
use std::time::{Duration, SystemTime};
use std::sync::{Arc, Mutex};
use std::thread;

const ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const PORT: u16 = 27491;
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
    let mut peers: Vec<Peer> = Vec::new();
    let mut files: Vec<File> = Vec::new();
    let peers_arc = Arc::new(Mutex::new(peers));
    let files_arc = Arc::new(Mutex::new(files));

    let peers_mutex1 = Arc::clone(&peers_arc);
    let files_mutex1 = Arc::clone(&files_arc);
    let recv_deam = thread::spawn(move || {
        receiver_deamon(peers_mutex1, files_mutex1)
    });
    let peers_mutex2 = Arc::clone(&peers_arc);
    let files_mutex2 = Arc::clone(&files_arc);
    let dist_deam = thread::spawn(move || {
        distribution_deamon(peers_mutex2, files_mutex2)
    });

    recv_deam.join().unwrap();
    dist_deam.join().unwrap();
}

fn receiver_deamon(peers: Arc<Mutex<Vec<Peer>>>, files: Arc<Mutex<Vec<File>>>){
    let listener = TcpListener::bind(SocketAddr::new(ADDR, PORT)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        register_peer_addr(&stream, &mut peers.lock().unwrap());
        let mut req_type = [0;1];
        let _ = stream.peek(&mut req_type).unwrap();
        match req_type[0] {
            b'p' => welcome_new_peer(stream, &peers.lock().unwrap()),
            b'd' => receive_data(stream, &mut files.lock().unwrap()),
            _ => stream.shutdown(Shutdown::Both).unwrap(),
        };
    }
}

fn distribution_deamon(peers: Arc<Mutex<Vec<Peer>>>, files: Arc<Mutex<Vec<File>>>){
    while true {
        thread::sleep(Duration::from_secs(2));
        println!("To be implemented: Forward files now.");
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