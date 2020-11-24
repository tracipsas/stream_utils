use crate::serialized_stream_error::SerializedStreamError;
use actix_web::{
    web::Bytes,
    ResponseError,
};
use futures::{
    task::{
        Context,
        Poll,
    },
    Stream,
};
use pin_project::pin_project;
use serde_utils::serde_str_enum;
use std::pin::Pin;

serde_str_enum!(
    LineEnding {
        CrLf("crlf"),
        Lf("lf"),
    }
);

impl Default for LineEnding {
    fn default() -> Self {
        LineEnding::CrLf
    }
}
impl LineEnding {
    pub fn value(&self) -> &'static str {
        match self {
            LineEnding::CrLf => "\r\n",
            LineEnding::Lf => "\n",
        }
    }
}

#[pin_project(project = HexStreamProject)]
pub struct HexStream<S> {
    is_first: bool,
    line_ending: LineEnding,
    buffer: String,
    #[pin]
    stream: S,
}

impl<S, E> HexStream<S>
where
    S: 'static + Sized + Stream<Item = Result<Vec<u8>, E>>,
    E: ResponseError,
{
    pub fn from(stream: S, line_ending: LineEnding) -> Self {
        HexStream {
            is_first: true,
            buffer: String::with_capacity(64 + 2),
            line_ending,
            stream,
        }
    }
}

impl<S, E> Stream for HexStream<S>
where
    S: 'static + Sized + Stream<Item = Result<Vec<u8>, E>>,
    E: ResponseError,
{
    type Item = Result<Bytes, SerializedStreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let HexStreamProject {
            is_first,
            buffer,
            line_ending,
            stream,
        } = self.project();
        match stream.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(bytes))) => {
                buffer.clear();
                if *is_first {
                    *is_first = false;
                } else {
                    buffer.push_str(line_ending.value());
                }
                unsafe {
                    match hex::encode_to_slice(bytes, buffer.as_bytes_mut()) {
                        Ok(_) => Poll::Ready(Some(Ok(Bytes::from(buffer.clone())))),
                        Err(e) => Poll::Ready(Some(Err(SerializedStreamError::HexError(e)))),
                    }
                }
            },
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(SerializedStreamError::SourceError(e))))
            },
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
