use std::net::SocketAddr;

use http_body_util::Empty;
use hyper::body::Bytes;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use websocket_sans_io::{
    FrameInfo, Opcode, WebsocketFrameDecoder, WebsocketFrameEncoder, WebsocketFrameEvent,
};

mod tokiort {
    // Based on https://github.com/hyperium/hyper/blob/master/benches/support/tokiort.rs

    #![allow(dead_code)]
    //! Various runtimes for hyper
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
        time::{Duration, Instant},
    };

    use hyper::rt::{Sleep, Timer};
    use pin_project_lite::pin_project;

    #[derive(Clone)]
    /// An Executor that uses the tokio runtime.
    pub struct TokioExecutor;

    impl<F> hyper::rt::Executor<F> for TokioExecutor
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        fn execute(&self, fut: F) {
            tokio::task::spawn(fut);
        }
    }

    /// A Timer that uses the tokio runtime.

    #[derive(Clone, Debug)]
    pub struct TokioTimer;

    impl Timer for TokioTimer {
        fn sleep(&self, duration: Duration) -> Pin<Box<dyn Sleep>> {
            Box::pin(TokioSleep {
                inner: tokio::time::sleep(duration),
            })
        }

        fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn Sleep>> {
            Box::pin(TokioSleep {
                inner: tokio::time::sleep_until(deadline.into()),
            })
        }

        fn reset(&self, sleep: &mut Pin<Box<dyn Sleep>>, new_deadline: Instant) {
            if let Some(sleep) = sleep.as_mut().downcast_mut_pin::<TokioSleep>() {
                sleep.reset(new_deadline.into())
            }
        }
    }

    struct TokioTimeout<T> {
        inner: Pin<Box<tokio::time::Timeout<T>>>,
    }

    impl<T> Future for TokioTimeout<T>
    where
        T: Future,
    {
        type Output = Result<T::Output, tokio::time::error::Elapsed>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            self.inner.as_mut().poll(context)
        }
    }

    // Use TokioSleep to get tokio::time::Sleep to implement Unpin.
    // see https://docs.rs/tokio/latest/tokio/time/struct.Sleep.html
    pin_project! {
        pub(crate) struct TokioSleep {
            #[pin]
            pub(crate) inner: tokio::time::Sleep,
        }
    }

    impl Future for TokioSleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.project().inner.poll(cx)
        }
    }

    impl Sleep for TokioSleep {}

    impl TokioSleep {
        pub fn reset(self: Pin<&mut Self>, deadline: Instant) {
            self.project().inner.as_mut().reset(deadline.into());
        }
    }

    pin_project! {
        #[derive(Debug)]
        pub struct TokioIo<T> {
            #[pin]
            inner: T,
        }
    }

    impl<T> TokioIo<T> {
        pub fn new(inner: T) -> Self {
            Self { inner }
        }

        pub fn inner(self) -> T {
            self.inner
        }
    }

    impl<T> hyper::rt::Read for TokioIo<T>
    where
        T: tokio::io::AsyncRead,
    {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            mut buf: hyper::rt::ReadBufCursor<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            let n = unsafe {
                let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
                match tokio::io::AsyncRead::poll_read(self.project().inner, cx, &mut tbuf) {
                    Poll::Ready(Ok(())) => tbuf.filled().len(),
                    other => return other,
                }
            };

            unsafe {
                buf.advance(n);
            }
            Poll::Ready(Ok(()))
        }
    }

    impl<T> hyper::rt::Write for TokioIo<T>
    where
        T: tokio::io::AsyncWrite,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            tokio::io::AsyncWrite::poll_write(self.project().inner, cx, buf)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            tokio::io::AsyncWrite::poll_flush(self.project().inner, cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            tokio::io::AsyncWrite::poll_shutdown(self.project().inner, cx)
        }

        fn is_write_vectored(&self) -> bool {
            tokio::io::AsyncWrite::is_write_vectored(&self.inner)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[std::io::IoSlice<'_>],
        ) -> Poll<Result<usize, std::io::Error>> {
            tokio::io::AsyncWrite::poll_write_vectored(self.project().inner, cx, bufs)
        }
    }

    impl<T> tokio::io::AsyncRead for TokioIo<T>
    where
        T: hyper::rt::Read,
    {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            tbuf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            //let init = tbuf.initialized().len();
            let filled = tbuf.filled().len();
            let sub_filled = unsafe {
                let mut buf = hyper::rt::ReadBuf::uninit(tbuf.unfilled_mut());

                match hyper::rt::Read::poll_read(self.project().inner, cx, buf.unfilled()) {
                    Poll::Ready(Ok(())) => buf.filled().len(),
                    other => return other,
                }
            };

            let n_filled = filled + sub_filled;
            // At least sub_filled bytes had to have been initialized.
            let n_init = sub_filled;
            unsafe {
                tbuf.assume_init(n_init);
                tbuf.set_filled(n_filled);
            }

            Poll::Ready(Ok(()))
        }
    }

    impl<T> tokio::io::AsyncWrite for TokioIo<T>
    where
        T: hyper::rt::Write,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            hyper::rt::Write::poll_write(self.project().inner, cx, buf)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            hyper::rt::Write::poll_flush(self.project().inner, cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            hyper::rt::Write::poll_shutdown(self.project().inner, cx)
        }

        fn is_write_vectored(&self) -> bool {
            hyper::rt::Write::is_write_vectored(&self.inner)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[std::io::IoSlice<'_>],
        ) -> Poll<Result<usize, std::io::Error>> {
            hyper::rt::Write::poll_write_vectored(self.project().inner, cx, bufs)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = "127.0.0.1:1234".parse()?;
    let s = tokio::net::TcpStream::connect(addr).await?;
    let s = tokiort::TokioIo::new(s);
    let b = hyper::client::conn::http1::Builder::new();
    let (mut sr, conn) = b.handshake::<_, Empty<Bytes>>(s).await?;
    tokio::spawn(async move {
        match conn.await {
            Ok(()) => (),
            Err(e) => {
                eprintln!("Error from connection: {e}");
            }
        }
    });

    let rq = hyper::Request::builder()
        .uri("/")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "wmnj1sVQ7pHv1bVR/wraDw==")
        .body(Empty::new())?;

    let resp = sr.send_request(rq).await?;

    let upg = hyper::upgrade::on(resp).await?;
    let Ok(parts) = upg.downcast::<tokiort::TokioIo<tokio::net::TcpStream>>() else {
        return Err("Failed to downcast".into());
    };
    let mut s = parts.io.inner();
    let debt = parts.read_buf;

    let mut buf = Vec::<u8>::with_capacity(debt.len().max(4096));
    buf.extend_from_slice(&debt[..]);

    let mut frame_decoder = WebsocketFrameDecoder::new();
    let mut frame_encoder = WebsocketFrameEncoder::new();
    let mut bufptr = 0;

    let mut error = false;

    println!("Connected to a WebSocket");

    loop {
        let bufslice = &mut buf[bufptr..];
        let ret = frame_decoder.add_data(bufslice)?;
        bufptr += ret.consumed_bytes;
        if let Some(ev) = ret.event {
            match ev {
                WebsocketFrameEvent::Start(mut fi) => {
                    if !fi.is_reasonable() {
                        println!("Unreasonable frame");
                        error = true;
                        break;
                    }
                    if fi.mask.is_some() {
                        println!("Masked frame while expected unmasked one");
                        error = true;
                        break;
                    }
                    println!(
                        "Frame {:?} with payload length {}{}",
                        fi.opcode,
                        fi.payload_length,
                        if fi.fin { "" } else { " (non-final)" }
                    );
                    if fi.opcode == Opcode::Ping {
                        fi.opcode = Opcode::Pong;
                    }
                    if fi.opcode == Opcode::ConnectionClose {
                        break;
                    }

                    fi.mask = Some(rand::thread_rng().gen());
                    let header = frame_encoder.start_frame(&fi);
                    s.write_all(&header[..]).await?;
                }
                WebsocketFrameEvent::PayloadChunk{original_opcode: _} => {
                    let payload_slice = &mut bufslice[ret.decoded_payload.unwrap()];
                    frame_encoder.transform_frame_payload(payload_slice);
                    s.write_all(payload_slice).await?;
                }
                WebsocketFrameEvent::End(_) => (),
            }
        }
        if ret.consumed_bytes == 0 && ret.event.is_none() {
            bufptr = 0;
            buf.resize(buf.capacity(), 0);
            let n = s.read(&mut buf[..]).await?;
            if n == 0 {
                println!("EOF");
                error = true;
                break;
            }
            buf.resize(n, 0);
        }
    }

    let header = frame_encoder.start_frame(&FrameInfo {
        opcode: Opcode::ConnectionClose,
        payload_length: 2,
        mask: Some(rand::thread_rng().gen()),
        fin: true,
        reserved: 0,
    });
    s.write_all(&header[..]).await?;
    let mut last_buf : [u8; 2] = if error { 1002u16 } else { 1000u16 }.to_be_bytes();
    frame_encoder.transform_frame_payload(&mut last_buf[..]);
    s.write_all(&last_buf[..]).await?;

    println!("Finished");

    Ok(())
}
