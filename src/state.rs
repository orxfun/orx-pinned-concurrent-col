use crate::{write_permit::WritePermit, PinnedConcurrentCol};
use orx_pinned_vec::{ConcurrentPinnedVec, PinnedVec};

/// Concurrent state of the collection.
pub trait ConcurrentState<T>
where
    Self: Sized,
{
    /// Determines whether or not new allocations of the pinned vector will be automatically zeroed out immediately after allocation.
    ///
    /// * If the method returns Some(f), new positions will be filled with f().
    /// * Otherwise, new positions will not be initialized.
    fn fill_memory_with(&self) -> Option<fn() -> T>;

    /// Creates a new state for the given `pinned_vec` which is to be wrapped by a [`PinnedConcurrentCol`].
    fn new_for_pinned_vec<P: PinnedVec<T>>(pinned_vec: &P) -> Self;

    /// Creates a new state for the given `con_pinned_vec` which is to be wrapped by a [`PinnedConcurrentCol`].
    fn new_for_con_pinned_vec<P: ConcurrentPinnedVec<T>>(con_pinned_vec: &P, len: usize) -> Self;

    /// Evaluates and returns the `WritePermit` for a request to write to the `idx`-th position of the given `col`.
    ///
    /// Note that [`PinnedConcurrentCol`] requires that only one growth can happen at any given point in time.
    /// When the result of this method is [`WritePermit::GrowThenWrite`]; i.e., when the caller thread is responsible for the growth,
    /// and if the state requires a handle, it must attain the handle with this call.
    /// This will be paired up with the `release_growth_handle` method, which will be called immediately after the allocation is completed.
    fn write_permit<P>(&self, col: &PinnedConcurrentCol<T, P, Self>, idx: usize) -> WritePermit
    where
        P: ConcurrentPinnedVec<T>;

    /// Evaluates and returns the `WritePermit` for a request to write `num_items` elements to sequential positions starting from `begin_idx`-th position of the given `col`.
    fn write_permit_n_items<P>(
        &self,
        col: &PinnedConcurrentCol<T, P, Self>,
        begin_idx: usize,
        num_items: usize,
    ) -> WritePermit
    where
        P: ConcurrentPinnedVec<T>,
    {
        let last_idx = begin_idx + num_items - 1;
        self.write_permit(col, last_idx)
    }

    /// If `write_permit` call returning [`WritePermit::GrowThenWrite`] grabs a growth handle, it must be released with this method.
    /// Otherwise, it might be an empty method.
    fn release_growth_handle(&self);

    /// Updates the state after writing values onto the range `begin_idx...end_idx`.
    fn update_after_write(&self, begin_idx: usize, end_idx: usize);

    /// Returns the debug information of the underlying pinned vector.
    #[allow(unused_variables)]
    fn pinned_vec_debug_info<P>(
        &self,
        col: &PinnedConcurrentCol<T, P, Self>,
        pinned_vec: &P,
    ) -> String
    where
        P: ConcurrentPinnedVec<T>,
    {
        "PinnedVec".to_string()
    }

    /// Tries to get the length of the underlying pinned vector which is written without a gap.
    /// Returns `None` if it is not known with certainty.
    fn try_get_no_gap_len(&self) -> Option<usize>;
}
