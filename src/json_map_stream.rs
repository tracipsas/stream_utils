use actix_web::{
    web::Bytes,
    ResponseError,
};
use futures::{
    Stream,
    TryStreamExt,
};
use pin_project::pin_project;
use serde::Serialize;
use sqlx::{
    postgres::PgPool,
    IntoArguments,
    Postgres,
};
use std::{
    mem,
    pin::Pin,
    ptr::NonNull,
    task::{
        Context,
        Poll,
    },
};

use crate::serialized_stream_error::SerializedStreamError;

use State::*;

#[pin_project(project = JsonMapStreamProject)]
pub struct JsonMapStream<O, K, E, I, F, G> {
    #[pin]
    pool: PgPool,
    state: State<K>,
    pinned_stream_opt: Option<Pin<Box<dyn 'static + Unpin + Stream<Item = Result<O, E>>>>>,
    remaining_queries: I,
    error_transform: F,
    post_transform: G,
}

impl<O, E, I, F, G, A, B, K, T, V, W> JsonMapStream<O, K, E, I, F, G>
where
    O: 'static + Send + Unpin + Serialize,
    E: 'static + ResponseError,
    F: 'static + Fn(sqlx::Error) -> E,
    A: 'static + Send + IntoArguments<'static, Postgres>,
    B: 'static + Send + Sync + Fn(<Postgres as sqlx::Database>::Row) -> Result<T, sqlx::Error>,
    I: 'static + Iterator<Item = (K, sqlx::query::Map<'static, Postgres, B, A>)>,
    K: 'static + Send + Unpin + Serialize,
    T: 'static + Send + Unpin,
    V: 'static + Unpin + Stream<Item = Result<T, dyn ResponseError>>,
    W: 'static + Unpin + Stream<Item = Result<O, dyn ResponseError>>,
    G: 'static + Fn(V) -> W,
{
    pub fn from<Im, Gm, Am, Bm, Tm, Vm, Wm, Km>(
        pool: PgPool,
        queries: Im,
        error_transform: F,
        post_transform: Gm,
    ) -> Pin<Box<JsonMapStream<O, Km, E, Im, F, Gm>>>
    where
        Am: 'static + Send + IntoArguments<'static, Postgres>,
        Bm: 'static
            + Send
            + Sync
            + Fn(<Postgres as sqlx::Database>::Row) -> Result<Tm, sqlx::Error>,
        Im: 'static + Iterator<Item = (Km, sqlx::query::Map<'static, Postgres, Bm, Am>)>,
        Km: 'static + Send + Unpin + Serialize,
        Tm: 'static + Send + Unpin,
        Vm: 'static + Unpin + Stream<Item = Result<Tm, dyn ResponseError>>,
        Wm: 'static + Unpin + Stream<Item = Result<O, dyn ResponseError>>,
        Gm: 'static + Fn(Vm) -> Wm,
    {
        let unpinned = JsonMapStream {
            pool,
            state: Init,
            pinned_stream_opt: None,
            remaining_queries: queries,
            error_transform,
            post_transform,
        };
        // I need to pin the struct here, to then get a pointer to pool which will
        // always be valid (because the struct can no longer be moved using the
        // pinned access, so the address of the inner pool struct will never
        // change) That's also why this struct returns a Pin<Box<Self>> and not
        // just Self
        let mut pinned = Box::pin(unpinned);
        pinned.as_mut().load_next_query_and_update_state();
        pinned
    }

    fn load_next_query_and_update_state(self: Pin<&mut Self>) -> () {
        // TODO: load_next_query_and_update_state should be executed if current stream
        let pool_raw_ptr = &self.pool as *const PgPool;
        let JsonMapStreamProject {
            pool: _,
            state,
            pinned_stream_opt,
            remaining_queries,
            error_transform,
            post_transform,
        } = self.project();

        match remaining_queries.next() {
            Some((key, new_query)) => {
                unsafe {
                    let _ = mem::replace(
                        pinned_stream_opt,
                        Some(Box::pin(post_transform(
                            new_query.fetch(&*pool_raw_ptr).map_err(error_transform),
                        ))),
                    );
                }
                match *state {
                    Init => *state = MustEmitGlobalOpeningAndItemOpening(key),
                    _ => *state = MustEmitItemClosingAndItemOpening(key),
                }
            },
            None => match *state {
                Init => *state = MustEmitGlobalOpeningAndGlobalClosing,
                _ => *state = MustEmitItemClosingAndGlobalClosing,
            },
        }
    }
}

enum State<K> {
    Init,
    MustEmitGlobalOpeningAndItemOpening(K),
    MustEmitItemClosingAndItemOpening(K),
    MustEmitGlobalOpeningAndGlobalClosing,
    MustEmitItemClosingAndGlobalClosing,
    MustEmitStreamItem { is_first: bool },
    Done,
}

fn indent(serialized_value: String, is_first: bool) -> String {
    let lines: Vec<String> = serialized_value.split('\n').collect();
    let mut text: String = lines.join("\n        ");
    if is_first {
        text.insert_str(0, "\n        ");
    } else {
        text.insert_str(0, ",\n        ");
    }
    text
}

impl<O, E, I, F, G, A, B, K, T, V, W> Stream for JsonMapStream<O, K, E, I, F, G>
where
    O: 'static + Send + Unpin + Serialize,
    E: 'static + ResponseError,
    F: 'static + Fn(sqlx::Error) -> E,
    A: 'static + Send + IntoArguments<'static, Postgres>,
    B: 'static + Send + Sync + Fn(<Postgres as sqlx::Database>::Row) -> Result<T, sqlx::Error>,
    I: 'static + Iterator<Item = (K, sqlx::query::Map<'static, Postgres, B, A>)>,
    K: 'static + Send + Unpin + Serialize,
    T: 'static + Send + Unpin,
    V: 'static + Unpin + Stream<Item = Result<T, dyn ResponseError>>,
    W: 'static + Unpin + Stream<Item = Result<O, dyn ResponseError>>,
    G: 'static + Fn(V) -> W,
{
    type Item = Result<Bytes, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let JsonMapStreamProject {
            pool: _,
            state,
            pinned_stream_opt,
            remaining_queries: _,
            error_transform: _,
            post_transform: _,
        } = self.project();
        match pinned_stream_opt.as_mut() {
            None => {
                // Should not happen, because pinned_stream is never set to None after
                // initialization
                Poll::Pending
            },
            Some(pinned_stream) => {
                let pinned_stream = pinned_stream.as_mut();
                match *state {
                    Init => {
                        // Should not happen, because state changes during initialization with
                        // the load_next_query_and_update_state call
                        Poll::Pending
                    },
                    MustEmitGlobalOpeningAndItemOpening(key) => {
                        *state = MustEmitStreamItem { is_first: true };
                        Poll::Ready(Some(match serde_json::to_string(&key) {
                            Ok(serialized_key) => {
                                let output = format!("{{\n    {}: [", serialized_key);
                                Ok(Bytes::from(output))
                            },
                            Err(e) => Err(SerializedStreamError::SerializationError(e)),
                        }))
                    },
                    MustEmitItemClosingAndItemOpening(key) => {
                        *state = MustEmitStreamItem { is_first: true };
                        Poll::Ready(Some(match serde_json::to_string(&key) {
                            Ok(serialized_key) => {
                                let output = format!("\n    ],\n    {}: [", serialized_key);
                                Ok(Bytes::from(output))
                            },
                            Err(e) => Err(SerializedStreamError::SerializationError(e)),
                        }))
                    },
                    MustEmitGlobalOpeningAndGlobalClosing => {
                        *state = Done;
                        Poll::Ready(Some(Ok(Bytes::from(format!("{{}}")))))
                    },
                    MustEmitItemClosingAndGlobalClosing => {
                        *state = Done;
                        Poll::Ready(Some(Ok(Bytes::from(format!("\n    ]\n}}")))))
                    },
                    MustEmitStreamItem { is_first } => match pinned_stream.poll_next(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(None) => {
                            self.load_next_query_and_update_state();
                            Poll::Pending
                        },
                        Poll::Ready(Some(Err(e))) => {
                            Poll::Ready(Some(Err(SerializedStreamError::SourceError(e))))
                        },
                        Poll::Ready(Some(Ok(value))) => {
                            if is_first {
                                *state = MustEmitStreamItem { is_first: false };
                            }
                            Poll::Ready(Some(match serde_json::to_string(&value) {
                                Ok(serialized_value) => {
                                    Ok(Bytes::from(indent(serialized_value, is_first)))
                                },
                                Err(e) => Err(SerializedStreamError::SerializationError(e)),
                            }))
                        },
                    },
                    Done => Poll::Ready(None),
                }
            },
        }
    }
}
