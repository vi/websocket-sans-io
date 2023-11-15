use std::io::Write;

use tungstenite::{protocol::Role, Message};
use websocket_sans_io::{FrameInfo, Opcode};

fn main() {
    let (tunstenite_end, mut sansio_end) = pipe::bipipe();
    std::thread::spawn(move || {
        let mut frame_encoder = websocket_sans_io::WebsocketFrameEncoder::new();
        let mut hello = *b"Hello, world\n";
        let header = frame_encoder.start_frame(&FrameInfo {
            opcode: Opcode::Text,
            payload_length: hello.len() as websocket_sans_io::PayloadLength,
            mask: Some(1234u32.to_be_bytes()),
            fin: true,
            reserved: 0,
        });
        sansio_end.write_all(&header[..]).unwrap();

        frame_encoder.transform_frame_payload(&mut hello[..]);
        sansio_end.write_all(&hello[..]).unwrap();
    });

    let mut tunstenite =
        tungstenite::protocol::WebSocket::from_raw_socket(tunstenite_end, Role::Server, None);
    let msg = tunstenite.read().unwrap();

    assert_eq!(msg, Message::Text("Hello, world\n".to_owned()));
}
