//! Low-level WebSocket framing library that does not use memory allocations or IO. Only frame headers are kept in memory, payload content is just shuffled around.
//!
//! It is also user's job to properly handle pings, closes, masking (including supplying masking keys) and joining frames into messages.
//!
//! It only implements WebSocket frames, not URLs or HTTP upgrades.

#![no_std]

pub struct WebSocketEncoder {
    buf: [u8; 3],
}
impl WebSocketEncoder {
    pub fn add_event<'a: 'c, 'b: 'c, 'c>(
        &'a mut self,
        e: WebsocketEvent,
        payload_chunk: Option<&'b mut [u8]>,
    ) -> Result<&'c [u8], Error> {
        if let Some(c) = payload_chunk {
            Ok(c)
        } else {
            Ok(&self.buf[..])
        }
    }
}

pub struct WebSocketDecoder {}

pub struct WebSocketDecoderAddDataResult<'a> {
    /// Data to be fed back into the next invocation of `add_data`.
    pub unprocessed_input_data: &'a mut [u8],
    /// Content of [`WebsocketEvent::DataFrameChunk`] or [`WebsocketEvent::ControlFrameChunk`], if any.
    pub decoded_payload: Option<&'a [u8]>,
    /// Emitted event, if any
    pub event: Option<WebsocketEvent>,
}

impl WebSocketDecoder {
    /// Call this function if any of the following conditions are met:
    /// 
    /// * When new incoming data is available on the socket
    /// * When previous invocation of `add_data` returned non-empty `unprocessed_input_data`.
    /// * When previous invocation of `add_data` returned non-`None` `event.
    pub fn add_data<'a, 'b>(
        &'a mut self,
        data: &'b [u8],
    ) -> WebSocketDecoderAddDataResult<'b> {
        todo!()
    }
}

pub enum Error {}

pub struct FrameInfo {
    pub payload_length: u64,
}

pub enum WebSocketMessageType {
    Binary,
    Text,
}

/// Events that [`WebSocketEncoder`] consume or [`WebSocketDecoder`] produce.
/// Does not contain actual payload data - content chunks are delivered (or supplied) as a separate argument
pub enum WebsocketEvent {
    /// Decode-only event, ignored by the encoder. Redundant, intended just for easier usage of the decoder.
    WebSocketMessageStart(WebSocketMessageType),
    DataFrameStart(FrameInfo),
    DataFrameChunk,
    DataFrameEnd(FrameInfo),
    /// Decode-only event, ignored by the encoder. Redundant, intended just for easier usage of the decoder.
    WebSocketMessageEnd(WebSocketMessageType),
    ControlFrameStart(FrameInfo),
    ControlFrameChunk,
    ControlFrameEnd(FrameInfo),
    Error(Error),
    /// Special encoder-only event that instructs encoder to roll back its state by specified number of output bytes,
    /// for cases when previous `add_event`'s output chunk was only partially written to a socket and this `add_event`
    /// should just produce the same bytes again.
    /// 
    /// Rewinding `DataFrameStart`s produce output data (some trailing part of the header).
    /// 
    /// Rewinding `DataFrameChunk` should produce no output data. You should rewind your input data, then use `DataFrameChunk` again.
    Rewind(usize),
}

/*impl core::error::Error for Error {

}*/

#[cfg(test)]
mod tests;
