//! 代理转发

use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

fn handle_client(mut stream: &TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer: Vec<u8> = Vec::new();
    stream.set_read_timeout(Some(Duration::from_millis(1)))?;
    loop {
        let mut tmp_buf = vec![0; 1024];
        match stream.read(&mut tmp_buf) {
            Ok(size) => buffer.extend(&tmp_buf[..size]),
            _ => break,
        }
    }

    let response = forward(&buffer)?;
    println!("返回{:?}",String::from_utf8(response.clone()));
    stream.write_all(&response)?;
    Ok(())
}

fn forward(buf: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    // but，如果代理需要认证怎么办，向 buf 中追加 Proxy-Authorization 头吗？我觉得是这样
    let mut stream = TcpStream::connect("60.188.79.111:20106")?;
    println!("连接");
    stream.write_all(&buf)?;
    // let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));

    let mut buffer: Vec<u8> = Vec::new();
    loop {
        let mut tmp_buf = vec![0; 1024];
        match stream.read(&mut tmp_buf) {
            Ok(0) => { println!("为0");break },
            Ok(size) => {println!("ok");buffer.extend(&tmp_buf[..size])},
            Err(e) => {
                println!("报错了");
                return Err(Box::new(e))
            }
        }
    }
    println!("返回{}",buffer.len());
    Ok(buffer)
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3128").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(s) => handle_client(&s).unwrap(),
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
}

#[test]
#[allow(unused)]
fn run() {
    main()
}