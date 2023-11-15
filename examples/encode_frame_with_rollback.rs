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
            mask: Some(1234567890u32.to_be_bytes()),
            fin: true,
            reserved: 0,
        });
        sansio_end.write_all(&header[..]).unwrap();

        frame_encoder.transform_frame_payload(&mut hello[..]);

        // Let's pretend we only can write 7 bytes here, then 
        // we need to forget the buffer and reconstruct it later.

        sansio_end.write_all(&hello[0..7]).unwrap();

        frame_encoder.rollback_payload_transform(hello.len() - 7);

        #[allow(dropping_copy_types)]
        drop(hello);

        // Now we returned and want to finish the writing.

        let mut hello_remembered = *b"Hello, world\n";
        let remaining_part = &mut hello_remembered[7..];

        frame_encoder.transform_frame_payload(remaining_part);
        sansio_end.write_all(&remaining_part[..]).unwrap();
    });

    let mut tunstenite =
        tungstenite::protocol::WebSocket::from_raw_socket(tunstenite_end, Role::Server, None);
    let msg = tunstenite.read().unwrap();

    assert_eq!(msg, Message::Text("Hello, world\n".to_owned()));
}
