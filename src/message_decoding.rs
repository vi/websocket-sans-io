use crate::{
    FrameDecoderError, WebSocketControlMessageType, WebSocketDataMessageType,
    WebsocketControlMessageEvent, WebsocketDataMessageEvent, WebsocketFrameDecoder,
    WebsocketFrameEvent, WebsocketMessageEvent, Opcode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageDecoderError {
    FrameError(FrameDecoderError),
    ProtocolError,
    MaskingPolicyViolation,
}

#[derive(Debug, Clone)]
pub struct WebsocketMessageDecoder {
    inner: WebsocketFrameDecoder,
    masking_policy: MaskingPolicy,
    continuation_state: ContinuationsState,
    inside_a_control_frame: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaskingPolicy {
    RequireMaskedFrames,
    RequireUnmaskedFrames,
    AcceptEither,
}

impl MaskingPolicy {
    fn check(&self, masked: bool) -> Result<(), MessageDecoderError> {
        let bad = match self {
            MaskingPolicy::RequireMaskedFrames => masked,
            MaskingPolicy::RequireUnmaskedFrames => !masked,
            MaskingPolicy::AcceptEither => false,
        };
        if bad {
            Err(MessageDecoderError::MaskingPolicyViolation)
        } else {
            Ok(())
        }
    }
}

pub struct WebsocketMessageDecoderAddDataResult {
    /// Data to be fed back into the next invocation of `add_data`.
    pub consumed_bytes: usize,
    /// Content of [`WebsocketDataMessageEvent::FrameChunk`] or [`WebsocketControlMessageEvent::FrameChunk`], if any, as index range of the input buffer.
    pub decoded_payload: Option<core::ops::Range<usize>>,
    /// Emitted event, if any
    pub event: Option<WebsocketMessageEvent>,
}

#[derive(Debug, Clone)]
enum ContinuationsState {
    NoOngoingContinuation,
    ThereIsUnfinishedMessage(WebSocketDataMessageType),
}

impl WebsocketMessageDecoder {
    pub const fn new(masking_policy: MaskingPolicy) -> WebsocketMessageDecoder {
        WebsocketMessageDecoder {
            inner: WebsocketFrameDecoder::new(),
            masking_policy,
            continuation_state: ContinuationsState::NoOngoingContinuation,
            inside_a_control_frame: false,
        }
    }
    #[inline]
    pub fn eof_valid(&self) -> bool {
        self.inner.eof_valid()
    }
    pub fn add_data<'a, 'b>(
        &'a mut self,
        data: &'b mut [u8],
    ) -> Result<WebsocketMessageDecoderAddDataResult, MessageDecoderError> {
        let ret = self
            .inner
            .add_data(data)
            .map_err(|e| MessageDecoderError::FrameError(e))?;
        assert!(
            ret.decoded_payload.is_some()
                == matches!(ret.event, Some(WebsocketFrameEvent::PayloadChunk))
        );
        let event = if let Some(frame_event) = ret.event {
            match frame_event {
                WebsocketFrameEvent::Start(fi) => Some({
                    if fi.reserved != 0 {
                        return Err(MessageDecoderError::ProtocolError);
                    }
                    self.masking_policy.check(fi.mask.is_some())?;

                    match fi.opcode {
                        Opcode::Continuation => match self.continuation_state {
                            ContinuationsState::NoOngoingContinuation => {
                                return Err(MessageDecoderError::ProtocolError)
                            }
                            ContinuationsState::ThereIsUnfinishedMessage(_) => {
                                WebsocketMessageEvent::Data(
                                    WebsocketDataMessageEvent::MorePayloadBytesWillFollow(
                                        fi.payload_length,
                                    ),
                                )
                            }
                        },
                        Opcode::Text => {
                            if matches!(self.continuation_state, ContinuationsState::ThereIsUnfinishedMessage(..)) {
                                return Err(MessageDecoderError::ProtocolError);
                            }
                            WebsocketMessageEvent::Data(WebsocketDataMessageEvent::Start(
                                WebSocketDataMessageType::Text,
                                fi.payload_length,
                            ))
                        }
                        Opcode::Binary => {
                            if matches!(self.continuation_state, ContinuationsState::ThereIsUnfinishedMessage(..)) {
                                return Err(MessageDecoderError::ProtocolError);
                            }
                            WebsocketMessageEvent::Data(WebsocketDataMessageEvent::Start(
                                WebSocketDataMessageType::Binary,
                                fi.payload_length,
                            ))
                        }
                        Opcode::ConnectionClose => {
                            if ! fi.fin {
                                return Err(MessageDecoderError::ProtocolError);
                            }
                            self.inside_a_control_frame = true;
                            WebsocketMessageEvent::Control(WebsocketControlMessageEvent::Start(
                                WebSocketControlMessageType::Close,
                                fi.payload_length,
                            ))
                        }
                        Opcode::Ping => {
                            if ! fi.fin {
                                return Err(MessageDecoderError::ProtocolError);
                            }
                            self.inside_a_control_frame = true;
                            WebsocketMessageEvent::Control(WebsocketControlMessageEvent::Start(
                                WebSocketControlMessageType::Ping,
                                fi.payload_length,
                            ))
                        }
                        Opcode::Pong => {
                            if ! fi.fin {
                                return Err(MessageDecoderError::ProtocolError);
                            }
                            self.inside_a_control_frame = true;
                            WebsocketMessageEvent::Control(WebsocketControlMessageEvent::Start(
                                WebSocketControlMessageType::Pong,
                                fi.payload_length,
                            ))
                        }
                        _ => return Err(MessageDecoderError::ProtocolError),
                    }
                }),
                WebsocketFrameEvent::PayloadChunk => Some(if self.inside_a_control_frame {
                    WebsocketMessageEvent::Control(WebsocketControlMessageEvent::PayloadChunk)
                } else {
                    WebsocketMessageEvent::Data(WebsocketDataMessageEvent::PayloadChunk)
                }),
                WebsocketFrameEvent::End(fi) => {
                    if fi.fin {
                        Some(match fi.opcode {
                            Opcode::Continuation => match self.continuation_state {
                                ContinuationsState::NoOngoingContinuation => {
                                    return Err(MessageDecoderError::ProtocolError)
                                }
                                ContinuationsState::ThereIsUnfinishedMessage(typ) => {
                                    WebsocketMessageEvent::Data(
                                        WebsocketDataMessageEvent::End(
                                            typ
                                        ),
                                    )
                                }
                            },
                            Opcode::Text => {
                                WebsocketMessageEvent::Data(WebsocketDataMessageEvent::End(
                                    WebSocketDataMessageType::Text,
                                ))
                            }
                            Opcode::Binary => {
                                WebsocketMessageEvent::Data(WebsocketDataMessageEvent::End(
                                    WebSocketDataMessageType::Binary,
                                ))
                            }
                            Opcode::ConnectionClose => {
                                self.inside_a_control_frame = false;
                                WebsocketMessageEvent::Control(WebsocketControlMessageEvent::End(
                                    WebSocketControlMessageType::Close,
                                ))
                            }
                            Opcode::Ping => {
                                self.inside_a_control_frame = false;
                                WebsocketMessageEvent::Control(WebsocketControlMessageEvent::End(
                                    WebSocketControlMessageType::Ping,
                                ))
                            }
                            Opcode::Pong => {
                                self.inside_a_control_frame = false;
                                WebsocketMessageEvent::Control(WebsocketControlMessageEvent::End(
                                    WebSocketControlMessageType::Pong,
                                ))
                            }
                            _ => unreachable!(),
                        })
                    } else {
                        match fi.opcode {
                            Opcode::Continuation => assert!(matches!(self.continuation_state, ContinuationsState::ThereIsUnfinishedMessage(..))),
                            Opcode::Text => self.continuation_state = ContinuationsState::ThereIsUnfinishedMessage(WebSocketDataMessageType::Text),
                            Opcode::Binary => self.continuation_state = ContinuationsState::ThereIsUnfinishedMessage(WebSocketDataMessageType::Binary),
                            _ => (),
                        }
                        None
                    }
                }
            }
        } else {
            None
        };
        Ok(WebsocketMessageDecoderAddDataResult {
            consumed_bytes: ret.consumed_bytes,
            decoded_payload: ret.decoded_payload,
            event,
        })
    }
}
