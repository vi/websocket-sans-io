extern crate std;
use std::eprintln;
use std::vec::Vec;
use proptest::collection::vec;
// Bring the macros and other important things into scope.
use proptest::prelude::*;
use std::format;

use std::println;

const DEBUG : bool = false;

fn roundtrip_frames(mut input: Vec<u8>) -> Result<Vec<u8>, TestCaseError> {
    let inputlen = input.len();
    let mut result = Vec::<u8>::with_capacity(inputlen);
    let mut ibuf = &mut input[..];

    let mut decoder = WebsocketFrameDecoder::new();
    let mut encoder = WebsocketFrameEncoder::new();

    if DEBUG {
        eprintln!("====================");
    }
    let mut cached_info : FrameInfo = FrameInfo::default();
    loop {
        let ret = decoder.add_data(ibuf).unwrap();
        if DEBUG {
            eprintln!("ioffset={}, ooffset={}, {:?}", inputlen - ibuf.len(), result.len(), &ret);
        }
        match ret.event {
            None => {
                if ret.consumed_bytes == 0 {
                    break;
                }
            }
            #[allow(unused_assignments)]
            Some(WebsocketFrameEvent::Start{frame_info:info, original_opcode:_}) => {
                cached_info = info;
                result.extend(encoder.start_frame(&info));
            }
            Some(WebsocketFrameEvent::PayloadChunk { original_opcode: for_opcode, data_range }) => {
                if cached_info.opcode != Opcode::Continuation {
                    prop_assert_eq!(for_opcode, cached_info.opcode);
                }
                let payload = &mut ibuf[data_range];
                encoder.transform_frame_payload(payload);
                result.extend_from_slice(payload);
            }
            Some(WebsocketFrameEvent::End{frame_info:info, original_opcode}) => {
                if cached_info.opcode != Opcode::Continuation {
                    prop_assert_eq!(original_opcode, cached_info.opcode);
                }
                prop_assert_eq!(info, cached_info);
            }
        }
        ibuf = &mut ibuf[ret.consumed_bytes..];
    }
    Ok(result)
}

use crate::Opcode;
use crate::{WebsocketFrameDecoder, WebsocketFrameEncoder, WebsocketFrameEvent, FrameInfo};
proptest! {
    #[cfg(feature="large_frames")]
    #[test]
    fn frame_roudtrip(s in byte_blob()) {
        let result1 = roundtrip_frames(s.0)?;
        let result2 = roundtrip_frames(result1.clone())?;

        for (i, (b1, b2)) in result2.iter().zip(result1.iter()).enumerate() {
            if b1 != b2 {
                println!("{i}'th byte should be {b1}, but is {b2}");
                break;
            }
        }
        prop_assert_eq!(ByteBlob(result1), ByteBlob(result2));
    }
}

fn byte_blob() -> impl Strategy<Value = ByteBlob> {
    vec(any::<u8>(), 50..80000).prop_map(|x|ByteBlob(x))
}


#[derive(PartialEq, Eq)]
struct ByteBlob(Vec<u8>);

impl std::fmt::Debug for ByteBlob {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        hex_fmt::HexFmt(&self.0).fmt(f)
    }
}
