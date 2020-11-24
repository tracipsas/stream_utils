#![feature(fn_traits)]
#![feature(unboxed_closures)]

pub mod json_array_stream;
pub use json_array_stream::JsonArrayStream;

pub mod owned_pool_stream;
pub use owned_pool_stream::OwnedPoolStream;

pub mod pseudo_stream;
pub use pseudo_stream::PseudoStream;

mod serialized_stream_error;
