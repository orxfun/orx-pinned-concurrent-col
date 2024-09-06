//! # orx-pinned-concurrent-col
//!
//! [![orx-pinned-concurrent-col crate](https://img.shields.io/crates/v/orx-pinned-concurrent-col.svg)](https://crates.io/crates/orx-pinned-concurrent-col)
//! [![orx-pinned-concurrent-col documentation](https://docs.rs/orx-pinned-concurrent-col/badge.svg)](https://docs.rs/orx-pinned-concurrent-col)
//!
//! A core data structure with a focus to enable high performance, possibly lock-free, concurrent collections using a [`PinnedVec`](https://crates.io/crates/orx-pinned-vec) as the underlying storage.
//!
//! Pinned vectors grow while keeping the already pushed elements pinned to their memory locations. This allows the following concurrency model.
//!
//! * Writing to the collection does not block. Multiple writes can happen concurrently.
//!   * However, `PinnedConcurrentCol` itself does not provide guarantees for race-free writing; and hence, the write methods are marked `unsafe`.
//!   * It is the responsibility of the wrapper to make sure that multiple writes or reading during write to the same position do not happen concurrently.
//! * Only one growth (capacity expansion) can happen at a given time.
//!   * If the underlying collection reaches its capacity and needs to grow, one and only one thread takes the responsibility to expand the vector.
//! * Growth does not block.
//!   * Writes to positions which are already within capacity are not blocked by the growth.
//!   * Writes to to-be-allocated positions wait only for the allocation to be completed; not any other task of the thread responsible for expansion.
//!
//! As clear from the properties, pinned concurrent collection aims to achieve high performance. It exposes the useful methods that can be used differently for different requirements and marks the methods which can lead to race conditions as `unsafe` by stating the underlying reasons. This enables building safe wrappers such as [`ConcurrentBag`](https://crates.io/crates/orx-concurrent-bag), [`ConcurrentOrderedBag`](https://crates.io/crates/orx-concurrent-ordered-bag) or [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
//!
//! ## Contributing
//!
//! Contributions are welcome! If you notice an error, have a question or think something could be improved, please open an [issue](https://github.com/orxfun/orx-pinned-concurrent-col/issues/new) or create a PR.
//!
//! ## License
//!
//! This library is licensed under MIT license. See LICENSE for details.

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
