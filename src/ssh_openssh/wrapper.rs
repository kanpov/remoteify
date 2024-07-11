use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

pub struct BoxedAsyncIoWrapper<T: AsyncRead + AsyncWrite + AsyncSeek> {
    pub inner: Pin<Box<T>>,
}

impl<T: AsyncRead + AsyncWrite + AsyncSeek> AsyncRead for BoxedAsyncIoWrapper<T> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        self.inner.as_mut().poll_read(cx, buf)
    }
}

impl<T: AsyncRead + AsyncWrite + AsyncSeek> AsyncWrite for BoxedAsyncIoWrapper<T> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        self.inner.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.inner.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.inner.as_mut().poll_shutdown(cx)
    }
}

impl<T: AsyncRead + AsyncWrite + AsyncSeek> AsyncSeek for BoxedAsyncIoWrapper<T> {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        self.inner.as_mut().start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        self.inner.as_mut().poll_complete(cx)
    }
}
