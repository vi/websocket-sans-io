use std::vec::Vec;

use std::vec;

use super::*;

extern crate std;

#[allow(unused)]
use std::dbg;

use pretty_assertions::assert_eq;

fn decode(input: &[u8], max_chunk_size : Option<usize>) -> (Vec<u8>, Vec<WebsocketFrameEvent>) {
    let mut input : Vec<u8> = input.into();
    let mut payload = Vec::new();
    let mut events = Vec::new();
    let mut d  = frame_decoding::WebsocketFrameDecoder::new();
    
    if let Some(mcs) = max_chunk_size {
        for chunk in input.chunks_mut(mcs) {
            decode_chunk(&mut d, chunk, &mut payload, &mut events);
        }
    } else {
        let ibuf = &mut input[..];
        decode_chunk(&mut d, ibuf, &mut payload, &mut events);
    }
    (payload, events)
}

fn decode_chunk(d: &mut frame_decoding::WebsocketFrameDecoder, mut ibuf: &mut [u8], payload: &mut Vec<u8>, events: &mut Vec<WebsocketFrameEvent>) {
    loop {
        //dbg!(ibuf.len());
        let ret = d.add_data(ibuf).unwrap();
        //dbg!(&ret);
        if let Some(WebsocketFrameEvent::PayloadChunk { ref data_range, .. }) = ret.event {
            payload.extend_from_slice(&ibuf[data_range.clone()]);    
        }
        ibuf = &mut ibuf[ret.consumed_bytes..];
        if ibuf.is_empty() && ret.event.is_none() {
            break;   
        }
        if let Some(ev) = ret.event {
            events.push(ev);
        }
    }
}

#[test]
fn decode_dummy() {
    assert_eq!(decode(b"", None), ((*b"").into(), vec![]));
}

#[test]
fn decode_simple_unmasked() {
    assert_eq!(decode(b"\x81\x05\x48\x65\x6c\x6c\x6f", None), 
    ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..5},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_unmasked_1bc() {
    assert_eq!(decode(b"\x81\x05\x48\x65\x6c\x6c\x6f", Some(1)), 
    ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_unmasked_2bc() {
    assert_eq!(decode(b"\x81\x05\x48\x65\x6c\x6c\x6f", Some(2)), 
    ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_unmasked_3bc() {
    assert_eq!(decode(b"\x81\x05\x48\x65\x6c\x6c\x6f", Some(3)), 
    ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..3},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: None, fin: true, reserved: 0 }},
    ]));
}


#[test]
fn decode_simple_masked() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", None), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..5},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_masked_1bc() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(1)), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_masked_2bc() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(2)), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_masked_3bc() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(3)), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..3},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_masked_5bc() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(5)), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..4},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_masked_6bc() {
    assert_eq!(decode(b"\x81\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(6)), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..5},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_simple_fragmented() {
    assert_eq!(decode(b"\x01\x03\x48\x65\x6c\x80\x02\x6c\x6f", None), ((*b"Hello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 3, mask: None, fin: false, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..3},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Text, payload_length: 3, mask: None, fin: false, reserved: 0 }},
        WebsocketFrameEvent::Start{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Continuation, payload_length: 2, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Text , data_range: 0..2},
        WebsocketFrameEvent::End{original_opcode: Opcode::Text, frame_info: FrameInfo { opcode: Opcode::Continuation, payload_length: 2, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_ping_pong() {
    assert_eq!(decode(b"\x89\x05\x48\x65\x6c\x6c\x6f\x8a\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", None), ((*b"HelloHello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Ping, frame_info: FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..5},
        WebsocketFrameEvent::End{original_opcode: Opcode::Ping, frame_info: FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::Start{original_opcode: Opcode::Pong, frame_info: FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..5},
        WebsocketFrameEvent::End{original_opcode: Opcode::Pong, frame_info: FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_ping_pong_1bc() {
    assert_eq!(decode(b"\x89\x05\x48\x65\x6c\x6c\x6f\x8a\x85\x37\xfa\x21\x3d\x7f\x9f\x4d\x51\x58", Some(1)), ((*b"HelloHello").into(), vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Ping, frame_info: FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Ping , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Ping, frame_info: FrameInfo { opcode: Opcode::Ping, payload_length: 5, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::Start{original_opcode: Opcode::Pong, frame_info: FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..1},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Pong , data_range: 0..1},
        WebsocketFrameEvent::End{original_opcode: Opcode::Pong, frame_info: FrameInfo { opcode: Opcode::Pong, payload_length: 5, mask: Some(*b"\x37\xfa\x21\x3d"), fin: true, reserved: 0 }},
    ]));
}

#[test]
fn decode_bin256() {
    let mut input : Vec<u8> = (*b"\x82\x7E\x01\x00").into();
    let zeroes = vec![0; 256];
    input.extend_from_slice(&zeroes[..]);
    assert_eq!(decode(&input, None), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 256, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..256},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 256, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[cfg(feature="large_frames")]
#[test]
fn decode_bin64k() {
    let mut input : Vec<u8> = (*b"\x82\x7F\x00\x00\x00\x00\x00\x01\x00\x00").into();
    let zeroes = vec![0; 65536];
    input.extend_from_slice(&zeroes[..]);
    std::assert_eq!(decode(&input, None), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..65536},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[cfg(feature="large_frames")]
#[test]
fn decode_bin64k_bc() {
    let mut input : Vec<u8> = (*b"\x82\x7F\x00\x00\x00\x00\x00\x01\x00\x00").into();
    let zeroes = vec![0; 65536];
    input.extend_from_slice(&zeroes[..]);
    std::assert_eq!(decode(&input, Some(32767)), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..32757},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..32767},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..12},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: None, fin: true, reserved: 0 }},
    ]));
}

#[cfg(feature="large_frames")]
#[test]
fn decode_bin64k_masked() {
    let mut input : Vec<u8> = (*b"\x82\xFF\x00\x00\x00\x00\x00\x01\x00\x00\x11\x22\x33\x44").into();
    let zeroes = vec![0; 65536];
    for _ in 0..(65536 / 4) {
        input.extend_from_slice(b"\x11\x22\x33\x44");
    }
    std::assert_eq!(decode(&input, None), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..65536},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
    ]));
}

#[cfg(feature="large_frames")]
#[test]
fn decode_bin64k_masked_chunks1() {
    let mut input : Vec<u8> = (*b"\x82\xFF\x00\x00\x00\x00\x00\x01\x00\x00\x11\x22\x33\x44").into();
    let zeroes = vec![0; 65536];
    for _ in 0..(65536 / 4) {
        input.extend_from_slice(b"\x11\x22\x33\x44");
    }
    std::assert_eq!(decode(&input, Some(65535)), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..65521},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..15},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
    ]));
}

#[cfg(feature="large_frames")]
#[test]
fn decode_bin64k_masked_chunks2() {
    let mut input : Vec<u8> = (*b"\x82\xFF\x00\x00\x00\x00\x00\x01\x00\x00\x11\x22\x33\x44").into();
    let zeroes = vec![0; 65536];
    for _ in 0..(65536 / 4) {
        input.extend_from_slice(b"\x11\x22\x33\x44");
    }
    std::assert_eq!(decode(&input, Some(2039)), (zeroes, vec![
        WebsocketFrameEvent::Start{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2025},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..2039},
        WebsocketFrameEvent::PayloadChunk { original_opcode: Opcode::Binary , data_range: 0..302},
        WebsocketFrameEvent::End{original_opcode: Opcode::Binary, frame_info: FrameInfo { opcode: Opcode::Binary, payload_length: 65536, mask: Some(*b"\x11\x22\x33\x44"), fin: true, reserved: 0 }},
    ]));
}
