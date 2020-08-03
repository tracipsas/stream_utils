use futures::Stream;
use pin_project::pin_project;
use serde::Serialize;
use std::{
    fmt::Display,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
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

impl<T, E, S> Stream for JsonArrayStream<S>
where
    T: 'static + Serialize,
    E: Display,
    S: 'static + Sized + Stream<Item = Result<T, E>>,
{
    type Item = Result<String, String>;

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
                            .map_err(|msg| format!("Serialization error: {}", msg)),
                        Err(source_error) => Err(format!("{}", source_error)),
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
