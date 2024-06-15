use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MyConState {
    pub initial_len: usize,
    pub initial_cap: usize,
}

impl MyConState {
    pub fn new(initial_len: usize, initial_cap: usize) -> Self {
        Self {
            initial_len,
            initial_cap,
        }
    }
}

impl ConcurrentState for MyConState {
    fn zero_memory(&self) -> bool {
        false
    }

    fn new_for_pinned_vec<T, P: PinnedVec<T>>(pinned_vec: &P) -> Self {
        Self::new(pinned_vec.len(), pinned_vec.capacity())
    }

    fn write_permit<T, P, S>(&self, col: &PinnedConcurrentCol<T, P, S>, idx: usize) -> WritePermit
    where
        P: PinnedVec<T>,
        S: ConcurrentState,
    {
        match idx.cmp(&col.capacity()) {
            Ordering::Less => WritePermit::JustWrite,
            Ordering::Equal => WritePermit::GrowThenWrite,
            Ordering::Greater => WritePermit::Spin,
        }
    }

    fn write_permit_n_items<T, P, S>(
        &self,
        col: &PinnedConcurrentCol<T, P, S>,
        begin_idx: usize,
        num_items: usize,
    ) -> WritePermit
    where
        P: PinnedVec<T>,
        S: ConcurrentState,
    {
        let capacity = col.capacity();
        let last_idx = begin_idx + num_items - 1;

        match (begin_idx.cmp(&capacity), last_idx.cmp(&capacity)) {
            (_, Ordering::Less) => WritePermit::JustWrite,
            (Ordering::Greater, _) => WritePermit::Spin,
            _ => WritePermit::GrowThenWrite,
        }
    }

    fn release_growth_handle(&self) {}

    fn update_after_write(&self, _: usize, _: usize) {}

    fn try_get_no_gap_len(&self) -> Option<usize> {
        None
    }
}

#[test]
fn pinned_vec_debug_info() {
    let col: PinnedConcurrentCol<String, _, MyConState> =
        PinnedConcurrentCol::with_doubling_growth();
    let state = MyConState::new(1, 2);
    assert_eq!(
        state.pinned_vec_debug_info(&col, &SplitVec::new()),
        "PinnedVec"
    );
}
