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
pub use frame_encoding::WebsocketFrameEncoder;
mod frame_decoding;
pub use frame_decoding::{FrameDecoderError,WebsocketFrameDecoder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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
    #[default]
    ReservedControlF = 0xF,
}

impl Opcode {
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
    pub fn is_control(&self) -> bool {
        ! self.is_data()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct FrameInfo {
    pub opcode: Opcode,
    pub payload_length: PayloadLength,
    pub mask: Option<[u8; 4]>,
    pub fin: bool,
    pub reserved: u8,
}

impl FrameInfo {
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

/// Events that [`WebSocketEncoder`] consume or [`WebSocketDecoder`] produce.
/// Does not contain actual payload data - content chunks are delivered (or supplied) as a separate argument
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WebsocketFrameEvent {
    Start(FrameInfo),
    PayloadChunk{ original_opcode: Opcode },
    End(FrameInfo),
}


#[cfg(feature = "large_frames")]
pub const MAX_HEADER_LENGTH: usize = 2 + 8 + 4;
#[cfg(not(feature = "large_frames"))]
pub const MAX_HEADER_LENGTH: usize = 2 + 2 + 4;

#[cfg(test)]
mod decoding_test;

#[cfg(test)]
mod frame_roundtrip_test;

