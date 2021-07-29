use actix_web::{
    web::Bytes,
    ResponseError,
};
use futures::Stream;
use pin_project::pin_project;
use serde::Serialize;
use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use crate::serialized_stream_error::SerializedStreamError;

#[pin_project(project = JsonMapStreamProject)]
pub struct JsonMapStream<S> {
    opening_emitted: bool,
    closing_emitted: bool,
    is_first: bool,
    #[pin]
    stream: S,
}

impl<S> From<S> for JsonMapStream<S>
where
    S: 'static + Sized + Stream,
{
    fn from(stream: S) -> Self {
        JsonMapStream {
            opening_emitted: false,
            closing_emitted: false,
            is_first: true,
            stream,
        }
    }
}

impl<T, E, S> Stream for JsonMapStream<S>
where
    T: 'static + Serialize,
    E: 'static + Sized + ResponseError,
    S: 'static + Sized + Stream<Item = Result<(String, T), E>>,
{
    type Item = Result<Bytes, SerializedStreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let JsonMapStreamProject {
            opening_emitted,
            closing_emitted,
            is_first,
            stream,
        } = self.project();
        if !*opening_emitted {
            *opening_emitted = true;
            Poll::Ready(Some(Ok("{\n    ".to_string().into())))
        } else if *closing_emitted {
            Poll::Ready(None)
        } else {
            match stream.poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Some(res)) => {
                    let mut line_res = res
                        .map_err(SerializedStreamError::SourceError)
                        .and_then(|(key, value)| {
                            serde_json::to_string(&key)
                                .map_err(SerializedStreamError::SerdeError)
                                .map(|serialized_key| (serialized_key, value))
                        })
                        .and_then(|(serialized_key, value)| {
                            serde_json::to_string(&value)
                                .map_err(SerializedStreamError::SerdeError)
                                .map(|value_serialized| {
                                    format!("{}: {}", serialized_key, value_serialized)
                                })
                        });
                    if let Ok(ref mut line) = line_res {
                        if *is_first {
                            *is_first = false;
                        } else {
                            line.insert_str(0, ",\n    ");
                        }
                    }
                    Poll::Ready(Some(line_res.map(Bytes::from)))
                },
                Poll::Ready(None) => {
                    *closing_emitted = true;
                    Poll::Ready(Some(Ok("\n}".to_string().into())))
                },
            }
        }
    }
}
