//! Low-level WebSocket framing library that does not use memory allocations or IO. Only frame headers are kept in memory, payload content is just shuffled around.
//!
//! It is also user's job to properly handle pings, closes, masking (including supplying masking keys) and joining frames into messages.
//!
//! It only implements WebSocket frames, not URLs or HTTP upgrades.

#![no_std]

use nonmax::NonMaxU8;

pub struct WebSocketFrameEncoder {
    buf: [u8; 3],
}
impl WebSocketFrameEncoder {
    pub fn add_event<'a: 'c, 'b: 'c, 'c>(
        &'a mut self,
        _e: WebsocketEvent,
        payload_chunk: Option<&'b mut [u8]>,
    ) -> Result<&'c [u8], Error> {
        if let Some(c) = payload_chunk {
            Ok(c)
        } else {
            Ok(&self.buf[..])
        }
    }
}

#[derive(Clone, Copy)]
struct SmallBufWithLen<const C: usize> {
    len: u8,
    data: [u8; C],
}

impl<const C: usize> SmallBufWithLen<C> {
    /// Take as much bytes as possible from the slice pointer, updating it in process
    fn slurp<'a, 'c>(&'c mut self, data: &'a mut [u8]) -> &'a mut [u8] {
        let offset = self.len as usize;
        let maxlen = (C - offset).min(data.len());
        self.data[offset..(offset+maxlen)].copy_from_slice(&data[..maxlen]);
        self.len += maxlen as u8;
        &mut data[maxlen..]
    }
    fn is_full(&self) -> bool {
        self.len as usize == C
    }
    const fn new() -> SmallBufWithLen<C> {
        SmallBufWithLen {
            len: 0,
            data: [0u8; C],
        }
    }
}

type PayloadLength = u64;
type FrameDecoderError = core::convert::Infallible;

/// Represents what data is expected to come next
#[derive(Clone, Copy)]
enum FrameDecodingState {
    HeaderBeginning(SmallBufWithLen<2>),
    PayloadLength16(SmallBufWithLen<2>),
    PayloadLength64(SmallBufWithLen<8>),
    MaskingKey(SmallBufWithLen<4>),
    PayloadData {
        phase: Option<NonMaxU8>,
        remaining: PayloadLength,
    },
}

pub struct WebSocketFrameDecoder {
    state: FrameDecodingState,
    mask: [u8; 4],
    basic_header: [u8; 2],
    payload_length: PayloadLength,
}

pub struct WebSocketDecoderAddDataResult<'a> {
    /// Data to be fed back into the next invocation of `add_data`.
    pub unprocessed_input_data: &'a mut [u8],
    /// Content of [`WebsocketEvent::DataFrameChunk`] or [`WebsocketEvent::ControlFrameChunk`], if any.
    pub decoded_payload: Option<&'a [u8]>,
    /// Emitted event, if any
    pub event: Option<WebsocketEvent>,
}

impl<'a> core::fmt::Debug for WebSocketDecoderAddDataResult<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebSocketDecoderAddDataResult")
            .field("unprocessed_input_data.len", &self.unprocessed_input_data.len())
            .field("decoded_payload.len", &self.decoded_payload.map(|x|x.len()))
            .field("event", &self.event)
            .finish()
    }
}

pub mod masking;

impl WebSocketFrameDecoder {
    fn get_opcode(&self) -> Opcode {
        use Opcode::*;
        match self.basic_header[0] & 0xF {
            0 => Continuation,
            1 => Text,
            2 => Binary,
            3 => ReservedData3,
            4 => ReservedData4,
            5 => ReservedData5,
            6 => ReservedData6,
            7 => ReservedData7,
            8 => ConnectionClose,
            9 => Ping,
            0xA => Pong,
            0xB => ReservedControlB,
            0xC => ReservedControlC,
            0xD => ReservedControlD,
            0xE => ReservedControlE,
            0xF => ReservedControlF,
            _ => unreachable!(),
        }
    }

    fn get_frame_info(&self, masked: bool) -> FrameInfo {
        FrameInfo {
            opcode: self.get_opcode(),
            payload_length: self.payload_length,
            mask: if masked { Some(self.mask) } else { None },
            fin: self.basic_header[0] & 0x80 == 0x80,
            reserved: (self.basic_header[0] & 0x70) >> 4,
        }
    }

    /// Call this function again if any of the following conditions are met:
    ///
    /// * When new incoming data is available on the socket
    /// * When previous invocation of `add_data` returned non-empty `unprocessed_input_data`.
    /// * When previous invocation of `add_data` returned non-`None` `event.
    pub fn add_data<'a, 'b>(
        &'a mut self,
        mut data: &'b mut [u8],
    ) -> Result<WebSocketDecoderAddDataResult<'b>, FrameDecoderError> {
        loop {
            
            macro_rules! return_dummy {
                () => {
                    return Ok(WebSocketDecoderAddDataResult {
                        unprocessed_input_data: data,
                        decoded_payload: None,
                        event: None,
                    });
                };
            }
            if data.len() == 0 && ! matches!(self.state, FrameDecodingState::PayloadData{remaining: 0, ..}) {
                return_dummy!();
            }
            macro_rules! try_to_fill_buffer_or_return {
                ($v:ident) => {
                    data = $v.slurp(data);
                    if !$v.is_full() {
                        assert!(data.is_empty());
                        return_dummy!();
                    }
                    let $v = $v.data;
                };
            }
            let mut length_is_ready = false;
            match self.state {
                FrameDecodingState::HeaderBeginning(ref mut v) => {
                    try_to_fill_buffer_or_return!(v);
                    self.basic_header = v;
                    match self.basic_header[1] & 0x7F {
                        0x7E => {
                            self.state = FrameDecodingState::PayloadLength16(SmallBufWithLen::new())
                        }
                        0x7F => {
                            self.state = FrameDecodingState::PayloadLength64(SmallBufWithLen::new())
                        }
                        x => {
                            self.payload_length = x.into();
                            length_is_ready = true;
                        }
                    };
                }
                FrameDecodingState::PayloadLength16(ref mut v) => {
                    try_to_fill_buffer_or_return!(v);
                    self.payload_length = u16::from_be_bytes(v).into();
                    length_is_ready = true;
                }
                FrameDecodingState::PayloadLength64(ref mut v) => {
                    try_to_fill_buffer_or_return!(v);
                    self.payload_length = u64::from_be_bytes(v);
                    length_is_ready = true;
                }
                FrameDecodingState::MaskingKey(ref mut v) => {
                    try_to_fill_buffer_or_return!(v);
                    self.mask = v;
                    self.state = FrameDecodingState::PayloadData {
                        phase: Some(NonMaxU8::default()),
                        remaining: self.payload_length,
                    };
                    return Ok(WebSocketDecoderAddDataResult {
                        unprocessed_input_data: data,
                        decoded_payload: None,
                        event: Some(WebsocketEvent::FrameStart(self.get_frame_info(true))),
                    });
                }
                FrameDecodingState::PayloadData {
                    phase,
                    remaining: 0,
                } => {
                    self.state = FrameDecodingState::HeaderBeginning(SmallBufWithLen::new());
                    return Ok(WebSocketDecoderAddDataResult {
                        unprocessed_input_data: data,
                        decoded_payload: None,
                        event: Some(WebsocketEvent::FrameEnd(
                            self.get_frame_info(phase.is_some()),
                        )),
                    });
                }
                FrameDecodingState::PayloadData {
                    ref mut phase,
                    ref mut remaining,
                } => {
                    let mut max_len = data.len();
                    if let Ok(remaining_usize) = usize::try_from(*remaining) {
                        max_len = max_len.min(remaining_usize);
                    }
                    let (payload_chunk, rest) = data.split_at_mut(max_len);

                    if let Some(phase) = phase {
                        let mut ph = phase.get();
                        masking::apply_mask(self.mask, payload_chunk, ph);
                        ph += payload_chunk.len() as u8;
                        *phase = NonMaxU8::new(ph & 0x03).unwrap();
                    }

                    *remaining -= max_len as PayloadLength;
                    return Ok(WebSocketDecoderAddDataResult {
                        unprocessed_input_data: rest,
                        decoded_payload: Some(payload_chunk),
                        event: Some(WebsocketEvent::FramePayloadChunk),
                    });
                }
            }
            if length_is_ready {
                if self.basic_header[1] & 0x80 == 0x80 {
                    self.state = FrameDecodingState::MaskingKey(SmallBufWithLen::new());
                } else {
                    self.state = FrameDecodingState::PayloadData {
                        phase: None,
                        remaining: self.payload_length,
                    };
                    return Ok(WebSocketDecoderAddDataResult {
                        unprocessed_input_data: data,
                        decoded_payload: None,
                        event: Some(WebsocketEvent::FrameStart(self.get_frame_info(false))),
                    });
                }
            }
        }
    }

    /// There is no incomplete WebSocket frame at this moment and EOF is valid here.
    ///
    /// This method is not related to [`Opcode::ConnectionClose`] in any way.
    pub fn eof_valid(&self) -> bool {
        matches!(self.state, FrameDecodingState::HeaderBeginning(..))
    }

    pub fn new() -> Self {
        WebSocketFrameDecoder {
            state: FrameDecodingState::HeaderBeginning(SmallBufWithLen::new()),
            mask: [0; 4],
            basic_header: [0; 2],
            payload_length: 0,
        }
    }
}

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

#[derive(Debug)]
pub enum Error {}

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

/*impl core::error::Error for Error {

}*/

#[cfg(test)]
mod tests;
