use std::io::Read;

use tungstenite::{protocol::Role, Message};
use websocket_sans_io::{FrameInfo, Opcode, WebsocketFrameEvent};

fn main() {
    let (tunstenite_end, mut sansio_end) = pipe::bipipe();
    std::thread::spawn(move || {
        let mut tunstenite =
            tungstenite::protocol::WebSocket::from_raw_socket(tunstenite_end, Role::Client, None);
        tunstenite
            .send(Message::Text("Hello, world\n".to_owned()))
            .unwrap();
    });

    let mut frame_decoder = websocket_sans_io::WebsocketFrameDecoder::new();
    let mut result = Vec::<u8>::new();
    let mut buf = [0u8; 1024];

    // This loop should handle multi-frame message and control messages interrupting stream of data frames,
    // but it does not reply to WebSocket pings
    'read_loop: loop {
        let n = sansio_end.read(&mut buf).unwrap();
        let mut processed_offset = 0;
        'decode_chunk_loop: loop {
            let unprocessed_part_of_buf = &mut buf[processed_offset..n];
            let ret = frame_decoder.add_data(unprocessed_part_of_buf).unwrap();
            processed_offset += ret.consumed_bytes;

            if ret.event.is_none() && ret.consumed_bytes == 0 {
                break 'decode_chunk_loop;
            }

            match ret.event {
                Some(WebsocketFrameEvent::PayloadChunk {
                    original_opcode: Opcode::Text,
                }) => {
                    result.extend_from_slice(&unprocessed_part_of_buf[0..ret.consumed_bytes]);
                }
                Some(WebsocketFrameEvent::End {
                    frame_info: FrameInfo { fin: true, .. },
                    original_opcode: Opcode::Text,
                }) => break 'read_loop,
                _ => (),
            }
        }
    }
    assert_eq!(result, b"Hello, world\n");
}
