use actix_web::{
    http::StatusCode,
    HttpResponse,
    ResponseError,
};
use std::fmt::{
    Debug,
    Display,
};

pub enum SerializedStreamError<E> {
    SourceError(E),
    SerializationError(serde_json::error::Error),
}
impl<E> Debug for SerializedStreamError<E>
where
    E: 'static + Sized + ResponseError,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializedStreamError::SourceError(e) => Debug::fmt(e, f),
            SerializedStreamError::SerializationError(e) => Debug::fmt(e, f),
        }
    }
}
impl<E> Display for SerializedStreamError<E>
where
    E: 'static + Sized + ResponseError,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializedStreamError::SourceError(e) => Display::fmt(e, f),
            SerializedStreamError::SerializationError(e) => Display::fmt(e, f),
        }
    }
}
impl<E> std::error::Error for SerializedStreamError<E> where E: 'static + Sized + ResponseError {}

impl<E> ResponseError for SerializedStreamError<E>
where
    E: 'static + Sized + ResponseError,
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
