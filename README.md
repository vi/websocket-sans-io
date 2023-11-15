Low-level WebSocket ([RFC 6455](https://www.rfc-editor.org/rfc/rfc6455)) library which implements WebSocket frame encoding and decoding.

* No memory allocations. Only minimal state is kept in memory, all payload content remains in user-supplied buffers. The crate is no_std-friendly.
* No input or output. It only helps you to turn raw bytes into sensible structures and back.
* Frame payloads may be divided into arbitrary chunks.
* No validation - you can set or access reserved bits or opcodes if needed.
* Encoder and decoder states are rather small. You can shrink the decoder further by opting out of `large_frames` crate feature.
* Masking should be reasonably fast and SIMD-friendly. You can adjust crate features to opt out the optimisation (for smaller code) or to adjust SIMD slice size.
* Encoder and decoder instances are const-initialisable.

It is also user's job to handle pings, HTTP upgrades, masking and close frames properly. There is no automatic assembling of messages from frames or splitting messages into frames. WebSocket text frames are handled the same way as binary frames - you need to convert to a string yourself.

# Examples

* [encode_frame](https://github.com/vi/websocket-sans-io/blob/main/examples/encode_frame.rs) - Encode one simple text WebSocket message and decode it with Tungstenite.
* [decode_frame](https://github.com/vi/websocket-sans-io/blob/main/examples/decode_frame.rs) - Encode one simple text message with Tungstenite and decode it with this library. Though no control or fragmented messages actually appears in this case, it tried to handle them properly to server as a template for other code.
* [mirror_client](https://github.com/vi/websocket-sans-io/blob/main/examples/mirror_client.rs) - Connect to a WebSocket server that is listening on `127.0.0.1:1234` and send back all frames which come from it, announcing each frame on console. Uses Tokio and hyper v1. Demonstrates how to validate incoming frames.
