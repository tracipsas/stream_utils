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
    SerdeError(serde_json::error::Error),
    HexError(hex::FromHexError),
}
impl<E> Debug for SerializedStreamError<E>
where
    E: 'static + Sized + ResponseError,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializedStreamError::SourceError(e) => Debug::fmt(e, f),
            SerializedStreamError::SerdeError(e) => Debug::fmt(e, f),
            SerializedStreamError::HexError(e) => Debug::fmt(e, f),
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
            SerializedStreamError::SerdeError(e) => Display::fmt(e, f),
            SerializedStreamError::HexError(e) => Display::fmt(e, f),
        }
    }
}
impl<E> std::error::Error for SerializedStreamError<E> where E: 'static + Sized + ResponseError {}

fn json_internal_server_error_response() -> HttpResponse {
    HttpResponse::InternalServerError()
        .set_header(actix_web::http::header::CONTENT_TYPE, "application/json")
        .body("{\"error\": \"Internal Server Error\"}")
}

impl<E> ResponseError for SerializedStreamError<E>
where
    E: 'static + Sized + ResponseError,
{
    fn status_code(&self) -> StatusCode {
        match self {
            SerializedStreamError::SourceError(e) => e.status_code(),
            SerializedStreamError::SerdeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SerializedStreamError::HexError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            SerializedStreamError::SourceError(e) => e.error_response(),
            SerializedStreamError::SerdeError(_) => json_internal_server_error_response(),
            SerializedStreamError::HexError(_) => HttpResponse::new(self.status_code()),
        }
    }
}
