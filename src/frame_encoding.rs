pub struct WebSocketFrameEncoder {
    pub(crate) _buf: [u8; 3],
}

pub type FrameEncoderError = core::convert::Infallible;
