use std::net::SocketAddr;

use http_body_util::Empty;
use hyper::body::Bytes;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use websocket_sans_io::{
    FrameInfo, Opcode, WebsocketFrameDecoder, WebsocketFrameEncoder, WebsocketFrameEvent,
};

#[path="../src/tokiort.rs"]
mod tokiort;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = "127.0.0.1:1234".parse()?;
    let s = tokio::net::TcpStream::connect(addr).await?;
    let s = tokiort::TokioIo::new(s);
    let b = hyper::client::conn::http1::Builder::new();
    let (mut sr, conn) = b.handshake::<_, Empty<Bytes>>(s).await?;
    tokio::spawn(async move {
        match conn.with_upgrades().await {
            Ok(()) => (),
            Err(e) => {
                eprintln!("Error from connection: {e}");
            }
        }
    });

    let rq = hyper::Request::builder()
        .uri("/")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "wmnj1sVQ7pHv1bVR/wraDw==")
        .body(Empty::new())?;

    let resp = sr.send_request(rq).await?;

    let upg = hyper::upgrade::on(resp).await?;
    let Ok(parts) = upg.downcast::<tokiort::TokioIo<tokio::net::TcpStream>>() else {
        return Err("Failed to downcast".into());
    };
    let mut s = parts.io.inner();
    let debt = parts.read_buf;

    let mut buf = Vec::<u8>::with_capacity(debt.len().max(4096));
    buf.extend_from_slice(&debt[..]);

    let mut frame_decoder = WebsocketFrameDecoder::new();
    let mut frame_encoder = WebsocketFrameEncoder::new();
    let mut bufptr = 0;

    let mut error = false;

    println!("Connected to a WebSocket");

    loop {
        let bufslice = &mut buf[bufptr..];
        let ret = frame_decoder.add_data(bufslice)?;
        bufptr += ret.consumed_bytes;
        if let Some(ref ev) = ret.event {
            match ev {
                WebsocketFrameEvent::Start{frame_info: mut fi, ..} => {
                    if !fi.is_reasonable() {
                        println!("Unreasonable frame");
                        error = true;
                        break;
                    }
                    if fi.mask.is_some() {
                        println!("Masked frame while expected unmasked one");
                        error = true;
                        break;
                    }
                    println!(
                        "Frame {:?} with payload length {}{}",
                        fi.opcode,
                        fi.payload_length,
                        if fi.fin { "" } else { " (non-final)" }
                    );
                    if fi.opcode == Opcode::Ping {
                        fi.opcode = Opcode::Pong;
                    }
                    if fi.opcode == Opcode::ConnectionClose {
                        break;
                    }

                    fi.mask = Some(rand::thread_rng().gen());
                    let header = frame_encoder.start_frame(&fi);
                    s.write_all(&header[..]).await?;
                }
                WebsocketFrameEvent::PayloadChunk{original_opcode: _} => {
                    let payload_slice = &mut bufslice[0..ret.consumed_bytes];
                    frame_encoder.transform_frame_payload(payload_slice);
                    s.write_all(payload_slice).await?;
                }
                WebsocketFrameEvent::End{..} => (),
            }
        }
        if ret.consumed_bytes == 0 && ret.event.is_none() {
            bufptr = 0;
            buf.resize(buf.capacity(), 0);
            let n = s.read(&mut buf[..]).await?;
            if n == 0 {
                println!("EOF");
                error = true;
                break;
            }
            buf.resize(n, 0);
        }
    }

    let header = frame_encoder.start_frame(&FrameInfo {
        opcode: Opcode::ConnectionClose,
        payload_length: 2,
        mask: Some(rand::thread_rng().gen()),
        fin: true,
        reserved: 0,
    });
    s.write_all(&header[..]).await?;
    let mut last_buf : [u8; 2] = if error { 1002u16 } else { 1000u16 }.to_be_bytes();
    frame_encoder.transform_frame_payload(&mut last_buf[..]);
    s.write_all(&last_buf[..]).await?;

    println!("Finished");

    Ok(())
}
