use async_std::net::{TcpListener, Ipv4Addr};
use async_std::net::TcpStream;
use async_std::task::spawn;
use async_std::task::block_on;
use futures::stream::StreamExt;
use futures::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryInto;
use async_std::io::copy;
use async_std::io::Error;

fn main() {
    block_on(async {
        accept_loop().await
    })
}

async fn accept_loop() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    while let Some(stream) = listener.incoming().next().await {
        let stream = stream.unwrap();
        spawn(async move {
            let res = handle_connection(stream).await;
            if let Err(_e) = res {
                println!("Oops!")
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Error> {
    let mut buf = [0_u8;1024];
    stream.read(&mut buf).await.unwrap();
    stream.write(&[5,0]).await.unwrap();
    let mut buff = [0_u8;1024];
    stream.read(&mut buff).await.unwrap();
    let cmd = buff[1];
    if cmd != 1 {
        return Ok(());
    }
    let hostname = get_address(&buff);
    let other_stream = TcpStream::connect(hostname).await.unwrap();
    stream.write(&[5,0,0,1,0,0,0,0,0,0]).await.unwrap();

    let (mut ir , mut iw) = stream.split();
    let (mut or , mut ow) = other_stream.split();

    // Download
    spawn(async move {
        copy(&mut or, &mut iw).await.unwrap();
    });

    // Upload
    spawn(async move {
        copy(&mut ir, &mut ow).await.unwrap();
    });

    Ok(())
}

enum AddrType {
    V4 = 0x01,
    Domain = 0x03,
    V6 = 0x04,
}

impl AddrType {
    /// Parse Byte to Command
    fn from(n: usize) -> Option<AddrType> {
        match n {
            1 => Some(AddrType::V4),
            3 => Some(AddrType::Domain),
            4 => Some(AddrType::V6),
            _ => None
        }
    }
}

fn get_address(buf: &[u8]) -> String {
    return match AddrType::from(buf[3] as usize).unwrap() {
        AddrType::Domain => {
            let x = buf[4];
            let n = usize::from(x);
            let address = &buf[5..n + 5];
            let mut hostname = std::str::from_utf8(address).unwrap().to_string();
            let port = read_be_u16(&mut &buf[n + 5..n + 7]);
            hostname.push_str(&":");
            hostname.push_str(&port.to_string());
            hostname
        }
        AddrType::V4 => {
            let address = &buf[4..8];
            let a = Ipv4Addr::new(address[0], address[1], address[2], address[3]);
            let mut hostname = a.to_string();
            let port = read_be_u16(&mut &buf[8..10]);
            hostname.push_str(&":");
            hostname.push_str(&port.to_string());
            hostname
        }
        AddrType::V6 => {
            String::from("test")
        }
    }
}

fn read_be_u16(input: &mut &[u8]) -> u16 {
    let (int_bytes, rest) = input.split_at(std::mem::size_of::<u16>());
    *input = rest;
    u16::from_be_bytes(int_bytes.try_into().unwrap())
}