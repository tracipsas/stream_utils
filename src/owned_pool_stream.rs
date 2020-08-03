use futures::Stream;
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

#[pin_project(project = OwnedPoolStreamProject)]
pub struct OwnedPoolStream<T> {
    #[pin]
    pool: PgPool,
    #[pin]
    stream: Option<Box<dyn 'static + Unpin + Stream<Item = Result<T, sqlx::Error>>>>,
}

impl<T> OwnedPoolStream<T>
where
    T: 'static + Send + Unpin,
{
    pub fn from<F, A>(
        pool: PgPool,
        query: sqlx::query::Map<'static, Postgres, F, A>,
    ) -> Pin<Box<Self>>
    where
        A: 'static + Send + IntoArguments<'static, Postgres>,
        F: 'static + Send + Sync + Fn(<Postgres as sqlx::Database>::Row) -> Result<T, sqlx::Error>,
    {
        let res = OwnedPoolStream { pool, stream: None };
        let mut boxed = Box::pin(res);
        let pool_ref = NonNull::from(&boxed.pool);

        unsafe {
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).stream =
                Some(Box::new(query.fetch(&*pool_ref.as_ptr())));
        }
        boxed
    }
}

impl<T> Stream for OwnedPoolStream<T>
where
    T: 'static,
{
    type Item = Result<T, sqlx::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.as_pin_mut() {
            None => Poll::Pending,
            Some(b) => b.poll_next(cx),
        }
    }
}
