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
    pin::Pin,
    ptr::NonNull,
    task::{
        Context,
        Poll,
    },
};
use actix_web::ResponseError;

#[pin_project(project = OwnedPoolStreamProject)]
pub struct OwnedPoolStream<T, E> {
    #[pin]
    pool: PgPool,
    #[pin]
    stream: Option<Box<dyn 'static + Unpin + Stream<Item = Result<T, E>>>>,
}

impl<T, E> OwnedPoolStream<T, E>
where
    T: 'static + Send + Unpin,
    E: ResponseError,
{
    pub fn from<K, A, F>(
        pool: PgPool,
        query: sqlx::query::Map<'static, Postgres, K, A>,
        error_transform: F,
    ) -> Pin<Box<Self>>
    where
        A: 'static + Send + IntoArguments<'static, Postgres>,
        K: 'static + Send + Sync + Fn(<Postgres as sqlx::Database>::Row) -> Result<T, sqlx::Error>,
        F: 'static + Fn(sqlx::Error) -> E,
    {
        let res = OwnedPoolStream { pool, stream: None };
        let mut boxed = Box::pin(res);
        let pool_ref = NonNull::from(&boxed.pool);

        unsafe {
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).stream =
                Some(Box::new(query.fetch(&*pool_ref.as_ptr()).map_err(error_transform)));
        }
        boxed
    }
}

impl<T, E> Stream for OwnedPoolStream<T, E>
where
    T: 'static,
{
    type Item = Result<T, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.as_pin_mut() {
            None => Poll::Pending,
            Some(b) => b.poll_next(cx),
        }
    }
}
