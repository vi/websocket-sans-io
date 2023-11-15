use crate::{PayloadLength, Opcode, FrameInfo, masking};

use nonmax::NonMaxU8;

/// When large_frames` crate feature is on (by default), any bytes can be decoded, so no error possible.
#[cfg(feature="large_frames")]
pub type FrameDecoderError = core::convert::Infallible;

/// When large_frames` crate feature is off (like now), WebSocket frame headers denoting large frames
/// produce this error.
#[allow(missing_docs)]
#[cfg(not(feature="large_frames"))]
#[derive(Debug,PartialEq, Eq, PartialOrd, Ord,Hash,Clone, Copy)]
pub enum FrameDecoderError {
    ExceededFrameSize,
}

#[derive(Clone, Copy, Debug)]
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

/// Represents what data is expected to come next
#[derive(Clone, Copy, Debug)]
enum FrameDecodingState {
    HeaderBeginning(SmallBufWithLen<2>),
    PayloadLength16(SmallBufWithLen<2>),
    #[cfg(feature="large_frames")]
    PayloadLength64(SmallBufWithLen<8>),
    MaskingKey(SmallBufWithLen<4>),
    PayloadData {
        phase: Option<NonMaxU8>,
        remaining: PayloadLength,
    },
}

impl Default for FrameDecodingState {
    fn default() -> Self {
        FrameDecodingState::HeaderBeginning(SmallBufWithLen::new())
    }
}

/// A low-level WebSocket frames decoder.
/// 
/// It is a push parser: you can add offer it bytes that come from a socket and it emites events.
/// 
/// You typically need two loops to process incoming data: outer loop reads chunks of data
/// from sockets, inner loop supplies this chunk to the decoder instance until no more events get emitted.
/// 
/// Example usage:
/// 
/// ```
#[doc=include_str!("../examples/decode_frame.rs")]
/// ```
/// 
/// Any sequence of bytes result in a some (sensial or not) [`WebsocketFrameEvent`]
/// sequence (exception: when `large_frames` crate feature is disabled).
/// 
/// You may want to validate it (e.g. using [`FrameInfo::is_reasonable`] method) before using.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebsocketFrameDecoder {
    state: FrameDecodingState,
    mask: [u8; 4],
    basic_header: [u8; 2],
    payload_length: PayloadLength,
    original_opcode: Opcode,
}

/// Return value of [`WebsocketFrameDecoder::add_data`] call.
#[derive(Debug,Clone)]
pub struct WebsocketFrameDecoderAddDataResult {
    /// Indicates how many bytes were consumed and should not be supplied again to
    /// the subsequent invocation of [`WebsocketFrameDecoder::add_data`].
    /// 
    /// When `add_data` procudes [`WebsocketFrameEvent::PayloadChunk`], it also indicated how many
    /// of the bytes in the buffer (starting from 0) should be used as a part of payload.
    pub consumed_bytes: usize,
    /// Emitted event, if any.
    pub event: Option<WebsocketFrameEvent>,
}

#[allow(missing_docs)]
/// Information that [`WebsocketFrameDecoder`] gives in return to bytes being fed to it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WebsocketFrameEvent {
    /// Indicates a frame is started.
    /// 
    /// `original_opcode` is the same as `frame_info.opcode`, except for
    /// [`Opcode::Continuation`] frames, for which it should refer to
    /// initial frame in sequence (i.e. [`Opcode::Text`] or [`Opcode::Binary`])
    Start{frame_info: FrameInfo, original_opcode: Opcode},

    /// Bytes which were supplied to [`WebsocketFrameDecoder::add_data`] are payload bytes,
    /// transformed for usage as a part of payload.
    /// 
    /// You should use [`WebsocketFrameDecoderAddDataResult::consumed_bytes`] to get actual
    /// buffer to be handled as content coming from the WebSocket.
    /// 
    /// Mind the `original_opcode` to avoid mixing content of control frames and data frames.
    PayloadChunk{ original_opcode: Opcode},

    /// Indicates that all `PayloadChunk`s for the given frame are delivered and the frame
    /// is ended.
    /// 
    /// You can watch for `frame_info.fin` together with checking `original_opcode` to know
    /// wnen WebSocket **message** (not just a frame) ends.
    /// 
    /// `frame_info` is the same as in [`WebsocketFrameEvent::Start`]'s `frame_info`.
    End{frame_info: FrameInfo, original_opcode: Opcode},
}

impl WebsocketFrameDecoder {
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

    /// Get frame info and original opcode
    fn get_frame_info(&self, masked: bool) -> (FrameInfo, Opcode) {
        let fi = FrameInfo {
            opcode: self.get_opcode(),
            payload_length: self.payload_length,
            mask: if masked { Some(self.mask) } else { None },
            fin: self.basic_header[0] & 0x80 == 0x80,
            reserved: (self.basic_header[0] & 0x70) >> 4,
        };
        let mut original_opcode = fi.opcode;
        if original_opcode==Opcode::Continuation {
            original_opcode = self.original_opcode;
        }
        (fi, original_opcode)
    }

    /// Add some bytes to the decoder and return events, if any.
    /// 
    /// Call this function again if any of the following conditions are met:
    ///
    /// * When new incoming data is available on the socket
    /// * When previous invocation of `add_data` returned nonzero [`WebsocketFrameDecoderAddDataResult::consumed_bytes`].
    /// * When previous invocation of `add_data` returned non-`None` [`WebsocketFrameDecoderAddDataResult::event`].
    /// 
    /// You may need call it with empty `data` buffer to get some final [`WebsocketFrameEvent::End`].
    /// 
    /// Input buffer needs to be mutable because it is also used to transform (unmask)
    /// payload content chunks in-place.
    pub fn add_data<'a, 'b>(
        &'a mut self,
        mut data: &'b mut [u8],
    ) -> Result<WebsocketFrameDecoderAddDataResult, FrameDecoderError> {
        let original_data_len = data.len();
        loop {
            macro_rules! return_dummy {
                () => {
                    return Ok(WebsocketFrameDecoderAddDataResult {
                        consumed_bytes: original_data_len - data.len(),
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
                    let opcode = self.get_opcode();
                    if opcode.is_data() && opcode != Opcode::Continuation {
                        self.original_opcode = opcode;
                    }
                    match self.basic_header[1] & 0x7F {
                        0x7E => {
                            self.state = FrameDecodingState::PayloadLength16(SmallBufWithLen::new())
                        }
                        #[cfg(feature="large_frames")]
                        0x7F => {
                            self.state = FrameDecodingState::PayloadLength64(SmallBufWithLen::new())
                        }
                        #[cfg(not(feature="large_frames"))] 0x7F => {
                            return Err(FrameDecoderError::ExceededFrameSize);
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
                #[cfg(feature="large_frames")]
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
                    let (frame_info, original_opcode) = self.get_frame_info(true);
                    return Ok(WebsocketFrameDecoderAddDataResult {
                        consumed_bytes: original_data_len - data.len(),
                        event: Some(WebsocketFrameEvent::Start{frame_info, original_opcode}),
                    });
                }
                FrameDecodingState::PayloadData {
                    phase,
                    remaining: 0,
                } => {
                    self.state = FrameDecodingState::HeaderBeginning(SmallBufWithLen::new());
                    let (fi, original_opcode) = self.get_frame_info(phase.is_some());
                    if fi.opcode.is_data() && fi.fin {
                        self.original_opcode = Opcode::Continuation;
                    }
                    return Ok(WebsocketFrameDecoderAddDataResult {
                        consumed_bytes: original_data_len - data.len(),
                        event: Some(WebsocketFrameEvent::End{frame_info: fi, original_opcode}
                            ),
                    });
                }
                FrameDecodingState::PayloadData {
                    ref mut phase,
                    ref mut remaining,
                } => {
                    let start_offset = original_data_len - data.len();
                    let mut max_len = data.len();
                    if let Ok(remaining_usize) = usize::try_from(*remaining) {
                        max_len = max_len.min(remaining_usize);
                    }
                    let (payload_chunk, _rest) = data.split_at_mut(max_len);

                    if let Some(phase) = phase {
                        let mut ph = phase.get();
                        masking::apply_mask(self.mask, payload_chunk, ph);
                        ph += payload_chunk.len() as u8;
                        *phase = NonMaxU8::new(ph & 0x03).unwrap();
                    }

                    *remaining -= max_len as PayloadLength;
                    let mut original_opcode = self.get_opcode();
                    if original_opcode == Opcode::Continuation {
                        original_opcode = self.original_opcode;
                    }
                    assert_eq!(start_offset, 0);
                    return Ok(WebsocketFrameDecoderAddDataResult {
                        consumed_bytes: max_len,
                        event: Some(WebsocketFrameEvent::PayloadChunk{original_opcode}),
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
                    let (frame_info, original_opcode) = self.get_frame_info(false);
                    return Ok(WebsocketFrameDecoderAddDataResult {
                        consumed_bytes: original_data_len - data.len(),
                        event: Some(WebsocketFrameEvent::Start{frame_info, original_opcode}),
                    });
                }
            }
        }
    }

    /// There is no incomplete WebSocket frame at this moment and EOF is valid here.
    ///
    /// This method is not related to [`Opcode::ConnectionClose`] in any way.
    #[inline]
    pub fn eof_valid(&self) -> bool {
        matches!(self.state, FrameDecodingState::HeaderBeginning(..))
    }

    /// Create new instance.
    #[inline]
    pub const fn new() -> Self {
        WebsocketFrameDecoder {
            state: FrameDecodingState::HeaderBeginning(SmallBufWithLen::new()),
            mask: [0; 4],
            basic_header: [0; 2],
            payload_length: 0,
            original_opcode: Opcode::Continuation,
        }
    }
}
