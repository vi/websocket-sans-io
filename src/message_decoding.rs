use crate::{WebsocketFrameDecoder, WebsocketMessageEvent, FrameDecoderError};

pub enum MessageDecoderError {
    FrameError(FrameDecoderError),
}

pub struct WebsocketMessageDecoder {
    inner: WebsocketFrameDecoder,
}

pub struct WebsocketMessageDecoderAddDataResult {
   /// Data to be fed back into the next invocation of `add_data`.
   pub consumed_bytes: usize,
   /// Content of [`WebsocketDataMessageEvent::FrameChunk`] or [`WebsocketControlMessageEvent::FrameChunk`], if any, as index range of the input buffer.
   pub decoded_payload: Option<core::ops::Range<usize>>,
   /// Emitted event, if any
   pub event: Option<WebsocketMessageEvent>,
}

impl WebsocketMessageDecoder {
    pub const fn new() -> WebsocketMessageDecoder {
        WebsocketMessageDecoder {
            inner: WebsocketFrameDecoder::new(),
        }
    }
    pub fn add_data<'a, 'b>(
        &'a mut self,
        mut data: &'b mut [u8],
    ) -> Result<WebsocketMessageDecoderAddDataResult, MessageDecoderError> {
        todo!()
    }
}
