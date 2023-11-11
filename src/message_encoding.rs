use tinyvec::ArrayVec;

use crate::{DataToBeWrittenToSocket, DummyMaskingFunction, PayloadLength, Role};
#[allow(unused_imports)]
use crate::{
    FrameDecoderError, MaskingFunction, Opcode, WebSocketControlMessageType,
    WebSocketDataMessageType, WebsocketControlMessageEvent, WebsocketDataMessageEvent,
    WebsocketFrameDecoder, WebsocketFrameEncoder, WebsocketFrameEvent, WebsocketMessageEvent,
    MAX_HEADER_LENGTH,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageEncoderError {
    UnfinishedFrameError,
    PayloadTransformationRequired,
}

const MAX_OUTPUT_QLEN: usize = 3;

#[derive(Debug, Clone)]
pub struct WebsocketMessageEncoder<MF>
where
    MF: MaskingFunction,
{
    inner: WebsocketFrameEncoder,
    masking_function: Option<MF>,
    payload_bytes_still_to_write: PayloadLength,
    output_queue: ArrayVec<[DataToBeWrittenToSocket; MAX_OUTPUT_QLEN]>,
}

impl WebsocketMessageEncoder<DummyMaskingFunction> {
    pub const fn new_server() -> WebsocketMessageEncoder<DummyMaskingFunction> {
        WebsocketMessageEncoder {
            inner: WebsocketFrameEncoder::new(),
            masking_function: None,
            payload_bytes_still_to_write: 0,
            output_queue: ArrayVec::from_array_empty([
                DataToBeWrittenToSocket::Inlined(ArrayVec::from_array_empty(
                    [0; MAX_HEADER_LENGTH],
                )),
                DataToBeWrittenToSocket::Inlined(ArrayVec::from_array_empty(
                    [0; MAX_HEADER_LENGTH],
                )),
                DataToBeWrittenToSocket::Inlined(ArrayVec::from_array_empty(
                    [0; MAX_HEADER_LENGTH],
                )),
            ]),
        }
    }
}

impl<MF: MaskingFunction> WebsocketMessageEncoder<MF> {
    pub fn new_client(masking_function: MF) -> WebsocketMessageEncoder<MF> {
        Self::new(Role::Client(masking_function))
    }
    pub fn new(role: Role<MF>) -> WebsocketMessageEncoder<MF> {
        WebsocketMessageEncoder {
            inner: WebsocketFrameEncoder::new(),
            masking_function: match role {
                Role::Server => None,
                Role::Client(x) => Some(x),
            },
            payload_bytes_still_to_write: 0,
            output_queue: Default::default(),
        }
    }

    pub fn add_event(
        &mut self,
        maybe_payload_chunk: Option<&mut [u8]>,
        event: WebsocketMessageEvent,
    ) -> Result<(), MessageEncoderError> {
        self.add_event_impl(None, maybe_payload_chunk, event)
    }

    pub fn add_event_with_immutable_payload(
        &mut self,
        maybe_payload_chunk: Option<&[u8]>,
        event: WebsocketMessageEvent,
    ) -> Result<(), MessageEncoderError> {
        self.add_event_impl(maybe_payload_chunk, None, event)
    }

    fn add_event_impl<'a>(
        &mut self,
        maybe_immutable_payload_chunk: Option<&'a [u8]>,
        maybe_mutable_payload_chunk: Option<&'a mut [u8]>,
        event: WebsocketMessageEvent,
    ) -> Result<(), MessageEncoderError> {
        assert!(
            (maybe_immutable_payload_chunk.is_some() || maybe_mutable_payload_chunk.is_some())
                == matches!(
                    event,
                    WebsocketMessageEvent::Data(WebsocketDataMessageEvent::PayloadChunk)
                        | WebsocketMessageEvent::Control(
                            WebsocketControlMessageEvent::PayloadChunk
                        )
                ),
            "Provide payload chunk if and only if you are using PayloadChunk input event"
        );
        match event {
            WebsocketMessageEvent::Data(x) => match x {
                WebsocketDataMessageEvent::Start(_, _) => todo!(),
                WebsocketDataMessageEvent::MorePayloadBytesWillFollow(_) => todo!(),
                WebsocketDataMessageEvent::PayloadChunk => {
                    if self.masking_function.is_some() {
                        if let Some(r) = maybe_mutable_payload_chunk {
                            self.inner.transform_frame_payload(r);
                        } else {
                            return Err(MessageEncoderError::PayloadTransformationRequired);
                        }
                    }
                    todo!()
                }
                WebsocketDataMessageEvent::End(_) => todo!(),
            },
            WebsocketMessageEvent::Control(x) => match x {
                WebsocketControlMessageEvent::Start(_, _) => todo!(),
                WebsocketControlMessageEvent::PayloadChunk => {
                    todo!()
                }
                WebsocketControlMessageEvent::End(_) => todo!(),
            },
        }
    }

    pub fn get_output(&mut self) -> Option<DataToBeWrittenToSocket> {
        if self.output_queue.is_empty() {
            return None
        }
        Some(self.output_queue.remove(0))
    }

    #[inline]
    pub fn mutable_input_needed(&self) -> bool {
        self.inner.transform_needed()
    }
}
