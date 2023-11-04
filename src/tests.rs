use std::vec::Vec;

use std::vec;

use super::*;

extern crate std;

#[allow(unused)]
use std::dbg;

use pretty_assertions::assert_eq;

fn decode(input: &[u8]) -> (Vec<u8>, Vec<WebsocketEvent>) {
    let mut input : Vec<u8> = input.into();
    let mut ibuf = &mut input[..];
    let mut payload = Vec::new();
    let mut events = Vec::new();
    let mut d  = WebSocketFrameDecoder::new();
    loop {
        //dbg!(ibuf.len());
        let ret = d.add_data(ibuf);
        //dbg!(&ret);
        ibuf = ret.unprocessed_input_data;
        if let Some(chunk) = ret.decoded_payload {
            payload.extend_from_slice(chunk);
        }
        if ibuf.is_empty() && ret.event.is_none() {
            break;   
        }
        if let Some(ev) = ret.event {
            events.push(ev);
        }
    }
    (payload, events)
}


#[test]
fn decode_dummy() {
    assert_eq!(decode(b""), ((*b"").into(), vec![]));
}

#[test]
fn decode_simple_unmasked() {
    assert_eq!(decode(b"\x81\x05\x48\x65\x6c\x6c\x6f"), 
    ((*b"Hello").into(), vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }),
    ]));
}


#[test]
fn decode_simple_masked() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58"), ((*b"Hello").into(), vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }),
    ]));
}

#[test]
fn decode_simple_fragmented() {
    assert_eq!(decode(b"\x01\x03\x48\x65\x6c\x80\x02\x6c\x6f"), ((*b"Hello").into(), vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Text, payload_length: 3, mask: None, fin: false, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Text, payload_length: 3, mask: None, fin: false, reserved: 0 }),
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Continuation, payload_length: 2, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Continuation, payload_length: 2, mask: None, fin: true, reserved: 0 }),
    ]));
}

#[test]
fn decode_ping_pong() {
    assert_eq!(decode(b"\x89\x05\x48\x65\x6c\x6c\x6f\x8a\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58"), ((*b"HelloHello").into(), vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }),
    ]));
}

#[test]
fn decode_bin256() {
    let mut input : Vec<u8> = (*b"\x82\x7E\x01\x00").into();
    let zeroes = vec![0; 256];
    input.extend_from_slice(&zeroes[..]);
    assert_eq!(decode(&input), (zeroes, vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Binary, payload_length: 256, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Binary, payload_length: 256, mask: None, fin: true, reserved: 0 }),
    ]));
}

#[test]
fn decode_bin64k() {
    let mut input : Vec<u8> = (*b"\x82\x7F\x00\x00\x00\x00\x00\x01\x00\x00").into();
    let zeroes = vec![0; 65536];
    input.extend_from_slice(&zeroes[..]);
    assert_eq!(decode(&input), (zeroes, vec![
        WebsocketEvent::FrameStart(FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }),
        WebsocketEvent::FramePayloadChunk,
        WebsocketEvent::FrameEnd(FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }),
    ]));
}


