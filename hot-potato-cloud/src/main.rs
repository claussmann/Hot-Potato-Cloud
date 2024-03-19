use std::net::{TcpListener, TcpStream, SocketAddr, Shutdown, IpAddr, Ipv4Addr};
use std::io::{prelude::*, BufReader};
use std::time::{Duration, SystemTime};
use std::sync::{Arc, Mutex};
use std::thread;
use rand::Rng;
use std::env;



const MAX_REPLICATION: u8 = 1;


struct File {
    data: String,
    last_seen: SystemTime,
    replicas_sent: u8,
}

struct Peer {
    addr: SocketAddr,
    last_seen: SystemTime,
}


fn main(){
    let mut peers: Vec<Peer> = Vec::new();
    let mut files: Vec<File> = Vec::new();
    let mut listen_addr = None;

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let first_peer = args[1].clone();
        listen_addr = join_network(&first_peer, &mut peers);
    }

    let peers_arc = Arc::new(Mutex::new(peers));
    let files_arc = Arc::new(Mutex::new(files));

    let peers_mutex1 = Arc::clone(&peers_arc);
    let files_mutex1 = Arc::clone(&files_arc);
    let recv_deam = thread::spawn(move || {
        receiver_deamon(listen_addr, peers_mutex1, files_mutex1)
    });
    let peers_mutex2 = Arc::clone(&peers_arc);
    let files_mutex2 = Arc::clone(&files_arc);
    let dist_deam = thread::spawn(move || {
        distribution_deamon(peers_mutex2, files_mutex2)
    });

    recv_deam.join().unwrap();
    dist_deam.join().unwrap();
}

fn receiver_deamon(listen_addr: Option<SocketAddr>, peers: Arc<Mutex<Vec<Peer>>>, files: Arc<Mutex<Vec<File>>>){
    let listener = match listen_addr {
        Some(x) => TcpListener::bind(x).unwrap(),
        None => TcpListener::bind("127.0.0.1:26001").unwrap(),
    };
    println!("Running on {}", listener.local_addr().unwrap());

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

fn distribution_deamon(peers_arc: Arc<Mutex<Vec<Peer>>>, files_arc: Arc<Mutex<Vec<File>>>){
    let mut rng = rand::thread_rng();
    loop {
        thread::sleep(Duration::from_secs(5));
        let mut peers = peers_arc.lock().unwrap();
        let mut files = files_arc.lock().unwrap();
        if peers.len() == 0 {
            println!("WARNING: I am lonely; No peers are known.");
            continue;
        }
        if files.len() == 0 {
            println!("WARNING: No files known.");
            continue;
        }
        let max_last_seen = 0;
        let mut oldest = 0;
        for i in 0..peers.len() {
            if peers[i].last_seen.elapsed().unwrap().as_millis() >= max_last_seen {
                oldest = i;
            }
        }
        if let Ok(mut stream) = TcpStream::connect_timeout(&peers[oldest].addr, Duration::from_secs(10)){
            stream.set_write_timeout(Some(Duration::from_secs(30))).unwrap();
            let rn = rng.gen_range(0..files.len());
            let string_to_send = format!("d{}\n", files[rn].data);
            if let Ok(sent) = stream.write(string_to_send.as_bytes()) {
                let mut br = BufReader::new(&mut stream);
                let mut response = String::new();
                let _ = br.read_line(&mut response).unwrap();
                if response == "OK" {
                    files[rn].replicas_sent += 1;
                    if files[rn].replicas_sent > MAX_REPLICATION {
                        println!("INFO: A file is removed due to max replicas reached.");
                        files.remove(rn);
                    }
                }
                else if response == "KNOWN" {
                    files[rn].last_seen = SystemTime::now();
                }
            }
            else {
                println!("INFO: A peer disconnected.");
                peers.remove(oldest);
            }
        }
        else {
            println!("INFO: A peer disconnected.");
            peers.remove(oldest);
        }
    }
}

fn register_peer_addr(stream: &TcpStream, peers: &mut Vec<Peer>){
    let peer_addr = stream.peer_addr().unwrap();
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

fn join_network(first_peer: &str, peers: &mut Vec<Peer>) -> Option<SocketAddr> {
    if let Ok(mut stream) = TcpStream::connect(first_peer){
        stream.set_write_timeout(Some(Duration::from_secs(30))).unwrap();
        stream.write("p".as_bytes()).unwrap();
        let mut br = BufReader::new(&mut stream);
        let mut response = String::new();
        let _ = br.read_line(&mut response).unwrap();
        println!("INFO: Joined peer to peer net.");
        // TODO: Receive peer addresses.
        return Some(stream.local_addr().unwrap());
    }
    println!("WARN: Failed to join peer to peer network.");
    return None;
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