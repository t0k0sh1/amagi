use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use flate2::{write::ZlibEncoder, Compression};


const SERVER: Token = Token(0);

struct KeyValueStore {
    store: HashMap<String, Vec<u8>>,
}

impl KeyValueStore {
    fn new() -> Self {
        KeyValueStore {
            store: HashMap::new(),
        }
    }

    fn set(&mut self, key: String, value: Vec<u8>) {
        self.store.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.store.get(key)
    }
}

fn compress_response(response: &str) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(response.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

fn handle_client(stream: &mut TcpStream, store: &mut KeyValueStore) -> bool {
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Ok(n) if n > 0 => {
            let request = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
            let parts: Vec<&str> = request.split_whitespace().collect();

            if request == "BYE" {
                // if client sends "BYE", close the connection
                stream.write_all("Goodbye!\n".as_bytes()).unwrap();
                return true;  // trueを返して接続を終了
            }

            if parts.len() == 3 && parts[0] == "SET" {
                // 圧縮データを16進数からデコードしてバイナリデータとして保存
                let value = hex::decode(parts[2]).unwrap_or_else(|_| vec![]);
                store.set(parts[1].to_string(), value);
                stream.write_all(&compress_response("OK\n")).unwrap();  // "OK" を返す
            } else if parts.len() == 2 && parts[0] == "GET" {
                if let Some(value) = store.get(parts[1]) {
                    // 圧縮データをそのまま返す
                    stream.write_all(value).unwrap();
                } else {
                    stream.write_all(&compress_response("NOT FOUND\n")).unwrap();  // 見つからなかった場合にメッセージを返す
                }
            } else {
                stream.write_all(&compress_response("ERROR\n")).unwrap();  // コマンドが不正な場合にエラーメッセージを返す
            }
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => {
            println!("Connection error: {}", e);
        }
    }
    false  // return false to keep the connection open
}

fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut listener = TcpListener::bind(addr)?;

    let mut poll = Poll::new()?;
    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    let mut events = Events::with_capacity(128);
    let mut store = KeyValueStore::new();
    let mut connections: HashMap<Token, TcpStream> = HashMap::new();
    let mut next_token = Token(SERVER.0 + 1);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    // accept new connections
                    loop {
                        match listener.accept() {
                            Ok((mut stream, _)) => {
                                let token = next_token;
                                next_token.0 += 1;
                                poll.registry()
                                    .register(&mut stream, token, Interest::READABLE.add(Interest::WRITABLE))?;
                                connections.insert(token, stream);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                println!("accept error: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
                token => {
                    // handle client connections
                    if let Some(mut stream) = connections.get_mut(&token) {
                        if handle_client(&mut stream, &mut store) {
                            // if handle_client returns true, close the connection
                            poll.registry().deregister(stream)?;
                            connections.remove(&token);
                        }
                    }
                }
            }
        }
    }
}
