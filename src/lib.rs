#![cfg_attr(not(feature="explicitly_aligned_masking"),forbid(unsafe_code))]
#![warn(missing_docs)]

#![doc=include_str!("../README.md")]
//! 
//! See [`WebsocketFrameEncoder`] and [`WebsocketFrameDecoder`] for continuation of the documentation.

#![no_std]

mod masking;

/// Apply WebSocket masking to the giben block of data.
/// 
/// `phase` is a number from 0 to 3, meaning zeroeth byte a `payload_chunk` should be 
/// masked with `phase`'s byte of `mask`.
/// 
/// Crate features `unoptimised_maskin`, `explicitly_aligned_masking` and `masking_slice_size_{4,8,16,32}` affect implementation of this function.
pub use masking::apply_mask;

/// Type alias for payload length. u64 by default, u16 when `large_frames` crate feature is off.
#[cfg(feature="large_frames")]
pub type PayloadLength = u64;

/// Type alias for payload length. u64 by default, u16 when `large_frames` crate feature is off.
#[cfg(not(feature="large_frames"))]
pub type PayloadLength = u16;

mod frame_encoding;
pub use frame_encoding::WebsocketFrameEncoder;
mod frame_decoding;
pub use frame_decoding::{FrameDecoderError,WebsocketFrameDecoder, WebsocketFrameEvent,WebsocketFrameDecoderAddDataResult};

/// WebSocket frame type.
/// 
/// Also includes reserved opcode types as `#[doc(hidden)]` items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Opcode {
    /// A continuation frame. Follows [`Opcode::Text`] or [`Opcode::Binary`] frames, extending content of it.
    /// Final `Continuation` frames has [`FrameInfo::fin`] set to true.
    /// 
    /// There may be control frames (e.g. [`Opcode::Ping`]) between initial data frame and continuation frames.
    Continuation = 0,
    /// First frame of a text WebSocket message.
    Text = 1,
    /// First frame of a binary WebSocket message.
    Binary = 2,
    #[doc(hidden)]
    ReservedData3 = 3,
    #[doc(hidden)]
    ReservedData4 = 4,
    #[doc(hidden)]
    ReservedData5 = 5,
    #[doc(hidden)]
    ReservedData6 = 6,
    #[doc(hidden)]
    ReservedData7 = 7,
    /// Last frame, indicating that WebSocket connection is now closed.
    /// You should close the socket upon receipt of this message.
    ConnectionClose = 8,
    /// WebSocket ping message. You should copy the payload to outgoing
    /// [`Opcode::Pong`] frame.
    Ping = 9,
    /// A reply to [`Opcode::Pong`] message.
    Pong = 0xA,
    #[doc(hidden)]
    ReservedControlB = 0xB,
    #[doc(hidden)]
    ReservedControlC = 0xC,
    #[doc(hidden)]
    ReservedControlD = 0xD,
    #[doc(hidden)]
    ReservedControlE = 0xE,
    #[doc(hidden)]
    #[default]
    ReservedControlF = 0xF,
}

impl Opcode {
    /// Check if this opcode is of a data frame.
    pub fn is_data(&self) -> bool {
        match self {
            Opcode::Continuation => true,
            Opcode::Text  => true,
            Opcode::Binary  => true,
            Opcode::ReservedData3 => true,
            Opcode::ReservedData4 => true,
            Opcode::ReservedData5 => true,
            Opcode::ReservedData6 => true,
            Opcode::ReservedData7 => true,
           _ => false,
        }
    }
    /// Check if this opcode is of a control frame.
    pub fn is_control(&self) -> bool {
        ! self.is_data()
    }
}

/// Information about WebSocket frame header.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct FrameInfo {
    /// Type of this WebSocket frame
    pub opcode: Opcode,
    /// Length of this frame's payload. Not to be confused with WebSocket 
    /// **message** length, which is unknown, unless [`FrameInfo::fin` ] is set.
    pub payload_length: PayloadLength,
    /// Whether this frame uses (or should use, for encoder) masking
    /// (and the mask used)
    pub mask: Option<[u8; 4]>,
    /// Whether this frame is a final frame and no [`Opcode::Continuation`] should follow
    /// to extend the content of it.
    pub fin: bool,
    /// Should be `0` unless some extensions are used.
    pub reserved: u8,
}

impl FrameInfo {
    /// Indicates that this frame looks like a normal, well-formed
    /// frame (not considering WebSocket extensions).
    /// 
    /// Does not check for valitity of a frame within a sequence of frames,
    /// e.g. for orphaned [`Opcode::Continuation`] frames or
    /// for unfinished prior messages.
    pub const fn is_reasonable(&self) -> bool {
        if self.reserved != 0 { return false; }
        match self.opcode {
            Opcode::Continuation => true,
            Opcode::Text => true,
            Opcode::Binary => true,
            Opcode::ConnectionClose => self.fin,
            Opcode::Ping => self.fin,
            Opcode::Pong => self.fin,
            _ => false,
        }
    }
}

/// Maximum number of bytes in a WebSocket frame header. Less if `large_frames` crate feature is off.
#[cfg(feature = "large_frames")]
pub const MAX_HEADER_LENGTH: usize = 2 + 8 + 4;

/// Maximum number of bytes in a WebSocket frame header. Less if `large_frames` crate feature is off.
#[cfg(not(feature = "large_frames"))]
pub const MAX_HEADER_LENGTH: usize = 2 + 2 + 4;

#[cfg(test)]
mod decoding_test;

#[cfg(test)]
mod frame_roundtrip_test;

