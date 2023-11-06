pub struct WebSocketFrameEncoder {
    pub(crate) buf: [u8; 3],
}

pub type FrameEncoderError = core::convert::Infallible;
