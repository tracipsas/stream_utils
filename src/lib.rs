#![feature(fn_traits)]
#![feature(unboxed_closures)]

pub mod json_array_stream;
pub use json_array_stream::JsonArrayStream;

pub mod json_map_stream;
pub use json_map_stream::JsonMapStream;

pub mod owned_pool_stream;
pub use owned_pool_stream::OwnedPoolStream;

pub mod pseudo_stream;
pub use pseudo_stream::PseudoStream;

pub mod hex_stream;
pub use hex_stream::{
    HexStream,
    LineEnding,
};

mod serialized_stream_error;
