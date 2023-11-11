use nonmax::NonMaxU8;
use tinyvec::ArrayVec;

use crate::{FrameInfo, MAX_HEADER_LENGTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebsocketFrameEncoder {
    mask: [u8; 4],
    phase: Option<NonMaxU8>,
}

impl WebsocketFrameEncoder {
    pub const fn new() -> WebsocketFrameEncoder {
        WebsocketFrameEncoder {
            mask: [0; 4],
            phase: None,
        }
    }

    #[inline]
    pub fn start_frame(&mut self, frame_info: &FrameInfo) -> ArrayVec<[u8; MAX_HEADER_LENGTH]> {
        if let Some(m) = frame_info.mask {
            self.mask = m;
            self.phase = Some(NonMaxU8::default());
        } else {
            self.phase = None;
        }
        encode_frame_header(frame_info)
    }

    #[inline]
    pub fn transform_frame_payload(&mut self, data: &mut [u8]) {
        if let Some(ref mut phase) = self.phase {
            let ph = phase.get();

            crate::masking::apply_mask(self.mask, data, ph);

            *phase = NonMaxU8::new( (ph + ((data.len() % 4) as u8)) % 4  ).unwrap();
        }
    }

    #[inline]
    pub fn rollback_payload_transform(&mut self, n_bytes: usize) {
        if let Some(ref mut phase) = self.phase {
            let modulo = (n_bytes % 4) as u8;
            let newvalue = (phase.get() + 4 - modulo) % 4;
            *phase = NonMaxU8::new(newvalue).unwrap();
        }
    }

    #[inline]
    pub const fn transform_needed(&self) -> bool {
        self.phase.is_some()
    }
}

#[inline]
pub fn encode_frame_header(frame_info: &FrameInfo) -> ArrayVec<[u8; MAX_HEADER_LENGTH]> {
    debug_assert!(frame_info.reserved & 0x7 == frame_info.reserved);

    let mut ret: ArrayVec<_> = ArrayVec::new();

    ret.push(
        if frame_info.fin { 0x80 } else { 0x00 }
            | (frame_info.reserved << 4)
            | (frame_info.opcode as u8),
    );
    let mut second_byte = if frame_info.mask.is_some() {
        0x80
    } else {
        0x00
    };
    match frame_info.payload_length {
        x if x <= 0x7D => {
            second_byte |= x as u8;
            ret.push(second_byte);
        }
        #[allow(unused_comparisons)]
        x if x <= 0xFFFF => {
            second_byte |= 0x7E;
            ret.push(second_byte);
            ret.extend((x as u16).to_be_bytes());
        }
        #[cfg(feature = "large_frames")]
        x => {
            second_byte |= 0x7F;
            ret.push(second_byte);
            ret.extend((x as u64).to_be_bytes());
        }
        #[cfg(not(feature = "large_frames"))]
        _ => unreachable!(),
    };

    if let Some(mask) = frame_info.mask {
        ret.extend(mask);
    }

    ret
}
