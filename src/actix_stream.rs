use actix_web::web::Bytes;
use futures::{
    Stream,
    StreamExt,
};
use pin_project::pin_project;
use std::{
    fmt::Display,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

pub type MapToActixStream<T, E, S> =
    futures::stream::Map<S, Box<dyn Fn(Result<T, E>) -> Result<Bytes, actix_web::Error>>>;

#[pin_project(project = ActixStreamProject)]
pub struct ActixStream<T, E, S> {
    #[pin]
    stream: MapToActixStream<T, E, S>,
}

impl<T, E, S> From<S> for ActixStream<T, E, S>
where
    T: Into<Bytes>,
    E: Display,
    S: 'static + Sized + Stream<Item = Result<T, E>>,
{
    fn from(stream: S) -> Self {
        ActixStream {
            stream: stream.map(Box::new(|res| {
                res.map(T::into).map_err(|msg| {
                    actix_web::error::ErrorInternalServerError(format!("Stream error: {}", msg))
                })
            })),
        }
    }
}

impl<T, E, S> Stream for ActixStream<T, E, S>
where
    T: Into<Bytes>,
    E: Display,
    S: 'static + Sized + Stream<Item = Result<T, E>>,
{
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let ActixStreamProject { stream } = self.project();
        stream.poll_next(cx)
    }
}
