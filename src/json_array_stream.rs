use futures::Stream;
use pin_project::pin_project;
use serde::Serialize;
use std::{
    fmt::{
        Display,
        Debug,
    },
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};
use actix_web::{
    ResponseError,
    http::StatusCode,
    HttpResponse,
};

#[pin_project(project = JsonArrayStreamProject)]
pub struct JsonArrayStream<S> {
    opening_emitted: bool,
    closing_emitted: bool,
    is_first: bool,
    #[pin]
    stream: S,
}

impl<S> From<S> for JsonArrayStream<S>
where
    S: 'static + Sized + Stream,
{
    fn from(stream: S) -> Self {
        JsonArrayStream {
            opening_emitted: false,
            closing_emitted: false,
            is_first: true,
            stream,
        }
    }
}

pub enum SerializedStreamError<E> {
    SourceError(E),
    SerializationError(serde_json::error::Error),
}
impl<E> Debug for SerializedStreamError<E>
    where E: 'static + Sized + ResponseError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializedStreamError::SourceError(e) => Debug::fmt(e, f),
            SerializedStreamError::SerializationError(e) => Debug::fmt(e, f),
        }
    }
}
impl<E> Display for SerializedStreamError<E>
    where E: 'static + Sized + ResponseError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializedStreamError::SourceError(e) => Display::fmt(e, f),
            SerializedStreamError::SerializationError(e) => Display::fmt(e, f),
        }
    }
}
impl<E> std::error::Error for SerializedStreamError<E>
    where E: 'static + Sized + ResponseError {}

impl<E> ResponseError for SerializedStreamError<E>
    where E: 'static + Sized + ResponseError
{
    fn status_code(&self) -> StatusCode {
        match self {
            SerializedStreamError::SourceError(e) => e.status_code(),
            SerializedStreamError::SerializationError(e) => e.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            SerializedStreamError::SourceError(e) => e.error_response(),
            SerializedStreamError::SerializationError(e) => e.error_response(),
        }
    }
}

impl<T, E, S> Stream for JsonArrayStream<S>
where
    T: 'static + Serialize,
    E: 'static + Sized + ResponseError,
    S: 'static + Sized + Stream<Item = Result<T, E>>,
{
    type Item = Result<String, SerializedStreamError<E>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let JsonArrayStreamProject {
            opening_emitted,
            closing_emitted,
            is_first,
            stream,
        } = self.project();
        if !*opening_emitted {
            *opening_emitted = true;
            Poll::Ready(Some(Ok("[\n    ".to_string())))
        } else if *closing_emitted {
            Poll::Ready(None)
        } else {
            match stream.poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Some(res)) => {
                    let mut new_res = match res {
                        Ok(value) => serde_json::to_string(&value)
                            .map_err(|serialization_error| SerializedStreamError::SerializationError(serialization_error)),
                        Err(source_error) => Err(SerializedStreamError::SourceError(source_error)),
                    };
                    new_res.iter_mut().for_each(|value_str| {
                        if *is_first {
                            *is_first = false;
                        } else {
                            value_str.insert_str(0, ",\n    ");
                        }
                    });
                    Poll::Ready(Some(new_res))
                },
                Poll::Ready(None) => {
                    *closing_emitted = true;
                    Poll::Ready(Some(Ok("\n]".to_string())))
                },
            }
        }
    }
}
