use std::io::{self, Write, Read};
use std::net::TcpStream;
use flate2::{Compression, write::ZlibEncoder, read::ZlibDecoder};

fn main() -> std::io::Result<()> {
    // connect to the server
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    println!("Connected to the server at 127.0.0.1:8080");

    loop {
        // accept user input
        print!("Enter command (SET key value, GET key, BYE to exit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let command = input.trim();

        // send the command to the server
        if command.starts_with("SET") {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() == 3 {
                let key = parts[1];
                let value = parts[2];
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(value.as_bytes())?;
                let compressed_data = encoder.finish()?;

                // send the compressed data to the server
                let compressed_command = format!("SET {} {}\n", key, hex::encode(compressed_data));
                stream.write_all(compressed_command.as_bytes())?;
            }
        }
        if command.starts_with("GET") {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() == 2 {
                let key = parts[1];
                let get_command = format!("GET {}\n", key);
                stream.write_all(get_command.as_bytes())?;
            }
        } else {
            stream.write_all(command.as_bytes())?;
        }

        if command == "BYE" {
            println!("Exiting...");
            break;
        }

        // read the server's response
        let mut response = vec![0; 512];
        let n = stream.read(&mut response)?;

        if n == 0 {
            println!("Connection closed by server.");
            break;
        }

        let mut decoder = ZlibDecoder::new(&response[..n]);
        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded)?;

        println!("Server response: {}", decoded);
    }

    Ok(())
}
