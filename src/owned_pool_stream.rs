use actix_web::ResponseError;
use futures::{
    Stream,
    TryStreamExt,
};
use pin_project::pin_project;
use sqlx::{
    postgres::PgPool,
    IntoArguments,
    Postgres,
};
use std::{
    mem,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

#[pin_project(project = OwnedPoolStreamProject)]
pub struct OwnedPoolStream<T, E> {
    #[pin]
    pool: PgPool,
    pinned_stream_opt: Option<Pin<Box<dyn 'static + Unpin + Stream<Item = Result<T, E>>>>>,
}

impl<T, E> OwnedPoolStream<T, E>
where
    T: 'static + Send + Unpin,
    E: 'static + ResponseError,
{
    pub fn from<B, A, F>(
        pool: PgPool,
        query: sqlx::query::Map<'static, Postgres, B, A>,
        error_transform: F,
    ) -> Pin<Box<Self>>
    where
        A: 'static + Send + IntoArguments<'static, Postgres>,
        B: 'static + Send + Sync + Fn(<Postgres as sqlx::Database>::Row) -> Result<T, sqlx::Error>,
        F: 'static + Fn(sqlx::Error) -> E,
    {
        let unpinned = OwnedPoolStream {
            pool,
            pinned_stream_opt: None,
        };
        // I need to pin the struct here, to then get a pointer to pool which will
        // always be valid (because the struct can no longer be moved using the
        // pinned access, so the address of the inner pool struct will never
        // change) That's also why this struct returns a Pin<Box<Self>> and not
        // just Self
        let mut pinned = Box::pin(unpinned);

        // I then get a pointer to the pool, to overcome lifetime limitations.
        // Because pool is owned by the OwnedPoolStream struct, as well as query, I know
        // that pool will always live as long as query, so query can be
        // constructed using a reference to pool. However, there is currently no
        // way in Rust to express that kind of self referencing lifetimes,
        // that's why I need this piece of unsafe code...
        let pool_raw_ptr = &pinned.pool as *const PgPool;

        // I then obtain a real mutable reference to the pinned_stream_opt field (which
        // is not pinned itself, but contains Option<Pin<Box<...>>
        let OwnedPoolStreamProject {
            pool: _,
            pinned_stream_opt,
        } = pinned.as_mut().project();

        // Eventually, I set the pinned_stream_opt field
        // Unsafe is required here only because I reference-dereference the raw pointer
        // to get a Rust reference with unlimited (or rather, unchecked)
        // lifetime
        unsafe {
            let _ = mem::replace(
                pinned_stream_opt,
                Some(Box::pin(
                    query.fetch(&*pool_raw_ptr).map_err(error_transform),
                )),
            );
        }

        pinned
    }
}

impl<T, E> Stream for OwnedPoolStream<T, E>
where
    T: 'static,
{
    type Item = Result<T, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let OwnedPoolStreamProject {
            pool: _,
            pinned_stream_opt,
        } = self.project();
        match pinned_stream_opt.as_mut() {
            None => {
                // Should not happen, because pinned_stream is never set to None after
                // initialization
                Poll::Pending
            },
            Some(pinned_stream) => {
                let pinned_stream = pinned_stream.as_mut();
                pinned_stream.poll_next(cx)
            },
        }
    }
}
