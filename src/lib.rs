#![doc = include_str!("../README.md")]
#![warn(
    missing_docs,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::missing_panics_doc,
    clippy::todo
)]
#![no_std]

extern crate alloc;

mod col;
mod common_traits;
mod errors;
mod mem_state;
mod new;
mod state;
mod write_permit;

/// Common relevant traits, structs, enums.
pub mod prelude;

pub use col::PinnedConcurrentCol;
pub use state::ConcurrentState;
pub use write_permit::WritePermit;
