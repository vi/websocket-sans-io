use nonmax::NonMaxU8;
use tinyvec::ArrayVec;

use crate::{FrameInfo, MAX_HEADER_LENGTH};

/// A low-level WebSocket frames decoder.
/// 
/// It lets to prepare frame headers and transform (mask) frame payloads when needed.
/// 
/// It does not validate that you supplied correct amount of payload bytes after headers or that headers make sense.
/// 
/// Example usage:
/// 
/// ```
#[doc=include_str!("../examples/encode_frame.rs")]
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WebsocketFrameEncoder {
    mask: [u8; 4],
    phase: Option<NonMaxU8>,
}

impl WebsocketFrameEncoder {
    /// Create new instance of WebsocketFrameEncoder
    pub const fn new() -> WebsocketFrameEncoder {
        WebsocketFrameEncoder {
            mask: [0; 4],
            phase: None,
        }
    }

    /// Serialize given frame header as bytes. You should write all those bytes to the socket 
    /// before starting to write payload contant (if any).
    /// 
    /// You can repeat the calls to `start_frame` to re-serialize the header of the same frame
    /// if you haven't yet called `transform_frame_payload`
    /// for that frame.
    /// 
    /// It does not validate the frame in any way and can allow you 
    /// to do nonsensial things such as starting conversation with `Continuation` frame
    /// or getting next frame header when current frame's payload is not completely written.
    /// 
    /// Use masked frames when you are client and unmasked frames when you are server.
    /// 
    /// Writing frame header with nonzero `frame_info.payload_length` means you are obligated to
    /// write this number of bytes before writing any new frame.
    /// 
    /// If you have large or unknown size of a WebSocket message, use frames with `fin=false` and
    /// [`crate::Opcode::Continuation`] frames with smaller payload lengths each.
    /// This also allows to interrupt the data transmissing to send a [`crate::Opcode::Ping`]
    /// or reply with a [`crate::Opcode::Pong`].
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

    /// Prepare this memory chunk to be transfitted to the socket as a part of WebSocket frame payload.
    /// 
    /// Call this after `start_frame`.
    /// 
    /// Chunks transformed by this method should be written to the socket in the same order as they
    /// are supplied to `transform_frame_payload`.
    /// 
    /// If you do not want to transmit the tranformed chunk or some of its trailing part, you can
    /// rollback the encoder state with [`WebsocketFrameEncoder::rollback_payload_transform`].
    #[inline]
    pub fn transform_frame_payload(&mut self, data: &mut [u8]) {
        if let Some(ref mut phase) = self.phase {
            let ph = phase.get();

            crate::masking::apply_mask(self.mask, data, ph);

            *phase = NonMaxU8::new( (ph + ((data.len() % 4) as u8)) % 4  ).unwrap();
        }
    }

    /// Undo transformation of this number of bytes.
    /// 
    /// Example:
    /// 
    /// ```
    #[doc=include_str!("../examples/encode_frame_with_rollback.rs")]
    /// ```
    #[inline]
    pub fn rollback_payload_transform(&mut self, n_bytes: usize) {
        if let Some(ref mut phase) = self.phase {
            let modulo = (n_bytes % 4) as u8;
            let newvalue = (phase.get() + 4 - modulo) % 4;
            *phase = NonMaxU8::new(newvalue).unwrap();
        }
    }

    /// Check if you can skip `transform_frame_payload` and just transfer payload as is.
    #[inline]
    pub const fn transform_needed(&self) -> bool {
        self.phase.is_some()
    }
}

/// Just encode the header to bytes without using any encoder instance.
/// 
/// May be useful when you do not need masking.
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
