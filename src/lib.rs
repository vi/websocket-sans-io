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

mod message_decoding;
pub use message_decoding::{WebsocketMessageDecoder,MessageDecoderError};
mod message_encoding;
pub use message_encoding::{WebsocketMessageEncoder,MessageEncoderError};
use tinyvec::ArrayVec;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WebSocketDataMessageType {
    Binary,
    Text,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WebSocketControlMessageType {
    Ping,
    Pong,
    Close,
}

/// Events that [`WebSocketEncoder`] consume or [`WebSocketDecoder`] produce.
/// Does not contain actual payload data - content chunks are delivered (or supplied) as a separate argument
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WebsocketFrameEvent {
    Start(FrameInfo),
    PayloadChunk,
    End(FrameInfo),
}


#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WebsocketDataMessageEvent {
    Start(WebSocketDataMessageType, PayloadLength),
    MorePayloadBytesWillFollow(PayloadLength),
    PayloadChunk,
    End(WebSocketDataMessageType),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WebsocketControlMessageEvent {
    Start(WebSocketControlMessageType, PayloadLength),
    PayloadChunk,
    End(WebSocketControlMessageType),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum WebsocketMessageEvent {
    Data(WebsocketDataMessageEvent),
    Control(WebsocketControlMessageEvent),
}

pub trait MaskingFunction {
    fn get_next_mask(&mut self) -> u32;
}
impl<T: FnMut() -> u32> MaskingFunction for T {
    fn get_next_mask(&mut self) -> u32 {
        (*self)()
    }
}
#[derive(Debug,Default,Clone, Copy)]
struct DummyMaskingFunction;
impl MaskingFunction for DummyMaskingFunction {
    fn get_next_mask(&mut self) -> u32 {
        panic!()
    }
}

#[derive(Debug, Clone)]
pub enum Role<MF: MaskingFunction> {
    Server,
    Client(MF),
}


#[cfg(feature = "large_frames")]
pub const MAX_HEADER_LENGTH: usize = 2 + 8 + 4;
#[cfg(not(feature = "large_frames"))]
pub const MAX_HEADER_LENGTH: usize = 2 + 2 + 4;

#[derive(Debug,Clone)]
pub enum DataToBeWrittenToSocket {
    PayloadChunkYouProvided(core::ops::Range<usize>),
    Inlined(ArrayVec<[u8; MAX_HEADER_LENGTH]>),
}

impl Default for DataToBeWrittenToSocket {
    fn default() -> Self {
        DataToBeWrittenToSocket::Inlined(Default::default())
    }
}

#[cfg(test)]
mod decoding_test;

#[cfg(test)]
mod frame_roundtrip_test;
