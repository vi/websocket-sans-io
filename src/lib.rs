//! Low-level WebSocket framing library that does not use memory allocations or IO. Only frame headers are kept in memory, payload content is just shuffled around.
//!
//! It is also user's job to properly handle pings, closes, masking (including supplying masking keys) and joining frames into messages.
//!
//! It only implements WebSocket frames, not URLs or HTTP upgrades.

#![no_std]

pub mod masking;


#[cfg(feature="large_frames")]
pub type PayloadLength = u64;
#[cfg(not(feature="large_frames"))]
pub type PayloadLength = u16;

mod frame_encoding;
pub use frame_encoding::{WebSocketFrameEncoder,FrameEncoderError};
mod frame_decoding;
pub use frame_decoding::{FrameDecoderError,WebSocketFrameDecoder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Opcode {
    Continuation = 0,
    Text = 1,
    Binary = 2,
    ReservedData3 = 3,
    ReservedData4 = 4,
    ReservedData5 = 5,
    ReservedData6 = 6,
    ReservedData7 = 7,
    ConnectionClose = 8,
    Ping = 9,
    Pong = 0xA,
    ReservedControlB = 0xB,
    ReservedControlC = 0xC,
    ReservedControlD = 0xD,
    ReservedControlE = 0xE,
    ReservedControlF = 0xF,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FrameInfo {
    pub opcode: Opcode,
    pub payload_length: PayloadLength,
    pub mask: Option<[u8; 4]>,
    pub fin: bool,
    pub reserved: u8,
}

pub enum WebSocketMessageType {
    Binary,
    Text,
}

/// Events that [`WebSocketEncoder`] consume or [`WebSocketDecoder`] produce.
/// Does not contain actual payload data - content chunks are delivered (or supplied) as a separate argument
#[derive(Debug, PartialEq, Eq)]
pub enum WebsocketEvent {
    FrameStart(FrameInfo),
    FramePayloadChunk,
    FrameEnd(FrameInfo),
}

#[cfg(test)]
mod tests;
