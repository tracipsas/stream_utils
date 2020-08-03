use futures::{
    ready,
    task::{
        Context,
        Poll,
    },
    Stream,
};
use pin_project::pin_project;
use std::pin::Pin;

#[pin_project(project = MapResProject)]
pub struct MapRes<S, F, G> {
    #[pin]
    stream: S,
    on_ok: F,
    on_err: G,
}

#[pin_project(project = MapOptionProject)]
pub struct MapOption<S, F> {
    #[pin]
    stream: S,
    on_some: F,
}

impl<S, F, G, T, E> Stream for MapRes<S, F, G>
where
    S: Stream<Item = Result<T, E>>,
    F: Fn<(T,)>,
    G: Fn<(E,)>,
{
    type Item = Result<F::Output, G::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let MapResProject {
            stream,
            on_ok,
            on_err,
        } = self.project();
        match ready!(stream.poll_next(cx)) {
            Some(Ok(value)) => Poll::Ready(Some(Ok(on_ok.call((value,))))),
            Some(Err(err)) => Poll::Ready(Some(Err(on_err.call((err,))))),
            None => Poll::Ready(None),
        }
    }
}

impl<S, F, T> Stream for MapOption<S, F>
where
    S: Stream<Item = Option<T>>,
    F: Fn<(T,)>,
{
    type Item = Option<F::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let MapOptionProject { stream, on_some } = self.project();
        match ready!(stream.poll_next(cx)) {
            Some(Some(value)) => Poll::Ready(Some(Some(on_some.call((value,))))),
            Some(None) => Poll::Ready(Some(None)),
            None => Poll::Ready(None),
        }
    }
}

pub trait StreamInnerMap: Stream {
    fn inner_res_map<T, E, F, G>(self, on_ok: F, on_err: G) -> MapRes<Self, F, G>
    where
        Self: Sized + Stream<Item = Result<T, E>>,
        F: Fn<(T,)>,
        G: Fn<(E,)>,
    {
        MapRes {
            stream: self,
            on_ok,
            on_err,
        }
    }

    fn inner_option_map<T, F>(self, on_some: F) -> MapOption<Self, F>
    where
        Self: Sized + Stream<Item = Option<T>>,
        F: Fn<(T,)>,
    {
        MapOption {
            stream: self,
            on_some,
        }
    }
}

impl<S: Stream> StreamInnerMap for S {}
