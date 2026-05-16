use clap::{Parser, Subcommand};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use raspi_fan::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "SOCKET", default_value = DEFAULT_SOCKET)]
    socket: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Status,
    Set { speed: String },
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Set { speed } => {
            let mut socket =
                UnixStream::connect(args.socket).expect("Could not connect to the socket");
            socket
                .write_all(speed.as_bytes())
                .expect("Could not write to the socket");
        }
        Commands::Status => {
            let socket = UnixStream::connect(args.socket);
            if let Ok(mut socket) = socket {
                socket
                    .write_all(b"GET mode")
                    .expect("Could not write to socket");
                let mut response = String::new();
                socket
                    .read_to_string(&mut response)
                    .expect("Could not read response from socket");
                println!("Fan mode is : {response}");
            } else {
                println!("Fan service is off");
            }
        }
    }
}
