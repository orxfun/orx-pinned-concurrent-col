use crate::{col::PinnedConcurrentCol, state::ConcurrentState};
use orx_fixed_vec::{FixedVec, PinnedVec};
use orx_split_vec::{Doubling, Linear, Recursive, SplitVec};

impl<T, S> PinnedConcurrentCol<T, SplitVec<T, Doubling>, S>
where
    T: Default,
    S: ConcurrentState,
{
    /// Creates a new concurrent bag by creating and wrapping up a new `SplitVec<T, Doubling>` as the underlying storage.
    pub fn with_doubling_growth() -> Self {
        Self::new_from_pinned(SplitVec::with_doubling_growth_and_fragments_capacity(32))
    }
}

impl<T, S> PinnedConcurrentCol<T, SplitVec<T, Recursive>, S>
where
    T: Default,
    S: ConcurrentState,
{
    /// Creates a new concurrent bag by creating and wrapping up a new `SplitVec<T, Recursive>` as the underlying storage.
    pub fn with_recursive_growth() -> Self {
        Self::new_from_pinned(SplitVec::with_recursive_growth_and_fragments_capacity(32))
    }
}

impl<T, S> PinnedConcurrentCol<T, SplitVec<T, Linear>, S>
where
    T: Default,
    S: ConcurrentState,
{
    /// Creates a new concurrent bag by creating and wrapping up a new `SplitVec<T, Linear>` as the underlying storage.
    ///
    /// Each fragment of the underlying split vector will have a capacity of `2 ^ constant_fragment_capacity_exponent`.
    ///
    /// `fragments_capacity` determines the initial `maximum_capacity` of the vector as follows: `maximum_capacity * 2 ^ constant_fragment_capacity_exponent`,
    /// which can be increased by `reserve_maximum_capacity` when necessary.
    ///
    /// # Panics
    ///
    /// Panics if `fragments_capacity == 0`.
    pub fn with_linear_growth(
        constant_fragment_capacity_exponent: usize,
        fragments_capacity: usize,
    ) -> Self {
        Self::new_from_pinned(SplitVec::with_linear_growth_and_fragments_capacity(
            constant_fragment_capacity_exponent,
            fragments_capacity,
        ))
    }
}

impl<T, S> PinnedConcurrentCol<T, FixedVec<T>, S>
where
    T: Default,
    S: ConcurrentState,
{
    /// Creates a new concurrent bag by creating and wrapping up a new `FixedVec<T>` as the underlying storage.
    ///
    /// # Safety
    ///
    /// Note that a `FixedVec` cannot grow; i.e., it has a hard upper bound on the number of elements it can hold, which is the `fixed_capacity`.
    ///
    /// Pushing to the vector beyond this capacity leads to "out-of-capacity" error.
    ///
    /// This maximum capacity can be accessed by [`PinnedConcurrentCol::maximum_capacity`] method.
    pub fn with_fixed_capacity(fixed_capacity: usize) -> Self {
        Self::new_from_pinned(FixedVec::new(fixed_capacity))
    }
}

// from
impl<T, P, S> From<P> for PinnedConcurrentCol<T, P, S>
where
    T: Default,
    P: PinnedVec<T>,
    S: ConcurrentState,
{
    /// Wraps the pinned vector and converts it into a pinned concurrent collection.
    fn from(pinned_vec: P) -> Self {
        Self::new_from_pinned(pinned_vec)
    }
}
