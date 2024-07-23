use orx_pinned_concurrent_col::*;
use orx_pinned_vec::{ConcurrentPinnedVec, PinnedVec};
use std::{
    cmp::Ordering,
    sync::atomic::{self, AtomicUsize},
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct MyConState {
    pub initial_len: usize,
    pub initial_cap: usize,
    pub len: AtomicUsize,
}

impl MyConState {
    pub fn new(initial_len: usize, initial_cap: usize) -> Self {
        Self {
            initial_len,
            initial_cap,
            len: initial_len.into(),
        }
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len.load(atomic::Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(crate) fn fetch_increment_len(&self, increment_by: usize) -> usize {
        self.len.fetch_add(increment_by, atomic::Ordering::AcqRel)
    }

    #[allow(dead_code)]
    pub fn set_final_len(&self, len: usize) {
        self.len.store(len, std::sync::atomic::Ordering::Relaxed);
    }
}

impl ConcurrentState for MyConState {
    fn zero_memory(&self) -> bool {
        false
    }

    fn new_for_pinned_vec<T, P: PinnedVec<T>>(pinned_vec: &P) -> Self {
        Self::new(pinned_vec.len(), pinned_vec.capacity())
    }

    fn new_for_con_pinned_vec<T, P: ConcurrentPinnedVec<T>>(
        con_pinned_vec: &P,
        len: usize,
    ) -> Self {
        Self::new(len, con_pinned_vec.capacity())
    }

    fn write_permit<T, P, S>(&self, col: &PinnedConcurrentCol<T, P, S>, idx: usize) -> WritePermit
    where
        P: ConcurrentPinnedVec<T>,
        S: ConcurrentState,
    {
        let capacity = col.capacity();

        match idx.cmp(&capacity) {
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
        P: ConcurrentPinnedVec<T>,
        S: ConcurrentState,
    {
        let capacity = col.capacity();
        let last_idx = begin_idx + num_items - 1;

        match (begin_idx.cmp(&capacity), last_idx.cmp(&capacity)) {
            (_, std::cmp::Ordering::Less) => WritePermit::JustWrite,
            (std::cmp::Ordering::Greater, _) => WritePermit::Spin,
            _ => WritePermit::GrowThenWrite,
        }
    }

    fn release_growth_handle(&self) {}

    fn update_after_write(&self, _: usize, _: usize) {}

    fn try_get_no_gap_len(&self) -> Option<usize> {
        Some(self.len())
    }
}
