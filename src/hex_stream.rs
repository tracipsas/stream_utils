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
            line_ending,
            stream,
        } = self.project();
        match stream.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(bytes))) => {
                let mut hex_bytes = hex::encode(&bytes);
                if *is_first {
                    *is_first = false;
                } else {
                    hex_bytes.push_str(line_ending.value());
                }
                Poll::Ready(Some(Ok(Bytes::from(hex_bytes))))
            },
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(SerializedStreamError::SourceError(e))))
            },
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
