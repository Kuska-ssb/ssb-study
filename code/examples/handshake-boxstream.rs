extern crate base64;
extern crate code;
extern crate crossbeam;

use crossbeam::thread;
use log::debug;
use sodiumoxide::crypto::{auth, sign::ed25519};
use std::env;
use std::io::{self};
use std::net::{TcpListener, TcpStream};

use code::handshake::{Handshake, SharedSecret};

fn usage(arg0: &str) {
    eprintln!(
        "Usage: {0} [client/server] OPTS
    client OPTS: addr server_pk
    server OPTS: addr",
        arg0
    );
}

fn print_shared_secret(shared_secret: &SharedSecret) {
    debug!("shared_secret {{");
    debug!("  ab: {}", hex::encode(shared_secret.ab.as_ref()));
    debug!("  aB: {}", hex::encode(shared_secret.aB.as_ref()));
    debug!("  Ab: {}", hex::encode(shared_secret.Ab.as_ref()));
    debug!("}}");
}

fn test_server(
    socket: TcpStream,
    net_id: auth::Key,
    pk: ed25519::PublicKey,
    sk: ed25519::SecretKey,
) -> io::Result<()> {
    let handshake = Handshake::new_server(&socket, &socket, net_id, pk, sk)
        .recv_client_hello()?
        .send_server_hello()?
        .recv_client_auth()?
        .send_server_accept()?;
    println!("Handshake complete! 💃");
    debug!("{:#?}", handshake);
    print_shared_secret(&handshake.state.shared_secret);

    let (mut box_stream_read, mut box_stream_write) =
        handshake.to_box_stream(0x8000).split_read_write();

    thread::scope(|s| {
        let handle = s.spawn(move |_| io::copy(&mut box_stream_read, &mut io::stdout()).unwrap());
        io::copy(&mut io::stdin(), &mut box_stream_write);
        handle.join().unwrap();
    })
    .unwrap();
    // box_stream.write(b"I'm the server")?;
    // box_stream.flush()?;
    // let mut buf = [0; 0x1000];
    // let n = box_stream.read(&mut buf)?;
    // println!("Received:");
    // io::stdout().write_all(&buf[..n])?;
    // println!();
    Ok(())
}

fn test_client(
    socket: TcpStream,
    net_id: auth::Key,
    pk: ed25519::PublicKey,
    sk: ed25519::SecretKey,
    server_pk: ed25519::PublicKey,
) -> io::Result<()> {
    let handshake = Handshake::new_client(&socket, &socket, net_id, pk, sk)
        .send_client_hello()?
        .recv_server_hello()?
        .send_client_auth(server_pk)?
        .recv_server_accept()?;
    println!("Handshake complete! 💃");
    debug!("{:#?}", handshake);
    print_shared_secret(&handshake.state.shared_secret);

    let (mut box_stream_read, mut box_stream_write) =
        handshake.to_box_stream(0x8000).split_read_write();

    thread::scope(|s| {
        let handle = s.spawn(move |_| io::copy(&mut box_stream_read, &mut io::stdout()).unwrap());
        io::copy(&mut io::stdin(), &mut box_stream_write);
        handle.join().unwrap();
    })
    .unwrap();
    // box_stream.write(b"I'm the client")?;
    // box_stream.flush()?;
    // let mut buf = [0; 0x1000];
    // let n = box_stream.read(&mut buf)?;
    // println!("Received:");
    // io::stdout().write_all(&buf[..n])?;
    // println!();
    Ok(())
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
        return Ok(());
    }
    let net_id_hex = "d4a1cb88a66f02f8db635ce26441cc5dac1b08420ceaac230839b755845a9ffb";
    let net_id = auth::Key::from_slice(&hex::decode(net_id_hex).unwrap()).unwrap();

    let (pk, sk) = ed25519::gen_keypair();
    let pk_b64 = base64::encode_config(&pk, base64::STANDARD);
    println!("Public key: {}", pk_b64);

    match args[1].as_str() {
        "client" => {
            if args.len() < 4 {
                usage(&args[0]);
                return Ok(());
            }
            let server_pk_buf = base64::decode_config(args[3].as_str(), base64::STANDARD).unwrap();
            let server_pk = ed25519::PublicKey::from_slice(&server_pk_buf).unwrap();
            let socket = TcpStream::connect(args[2].as_str())?;
            test_client(socket, net_id, pk, sk, server_pk)
        }
        "server" => {
            if args.len() < 3 {
                usage(&args[0]);
                return Ok(());
            }
            let listener = TcpListener::bind(args[2].as_str()).unwrap();
            println!(
                "Listening for a handshake via TCP at {} ...",
                args[2].as_str()
            );
            let (socket, addr) = listener.accept()?;
            println!("Client {} connected", addr);
            test_server(socket, net_id, pk, sk)
        }
        _ => Ok(()),
    }
}
