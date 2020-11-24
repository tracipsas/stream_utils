use actix_web::ResponseError;
use futures::{
    Future,
    Stream,
};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

#[pin_project(project = PseudoStreamProject)]
pub struct PseudoStream<F> {
    emitted: bool,
    #[pin]
    future: F,
}

impl<F> From<F> for PseudoStream<F>
where
    F: 'static + Sized + Future,
{
    fn from(future: F) -> Self {
        PseudoStream {
            emitted: false,
            future,
        }
    }
}

impl<T, E, F> Stream for PseudoStream<F>
where
    T: 'static,
    E: 'static + Sized + ResponseError,
    F: 'static + Sized + Future<Output = Result<T, E>>,
{
    type Item = Result<T, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let PseudoStreamProject { emitted, future } = self.project();
        if *emitted {
            Poll::Ready(None)
        } else {
            match future.poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(value) => {
                    *emitted = true;
                    Poll::Ready(Some(value))
                },
            }
        }
    }
}
