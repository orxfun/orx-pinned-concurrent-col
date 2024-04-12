use orx_fixed_vec::PinnedVec;
use std::{
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

pub(crate) struct CapacityState {
    capacity: AtomicUsize,
    maximum_capacity: UnsafeCell<usize>,
}

impl CapacityState {
    // new
    pub fn new_for_pinned_vec<T, P: PinnedVec<T>>(pinned_vec: &P) -> Self {
        Self {
            capacity: pinned_vec.capacity().into(),
            maximum_capacity: pinned_vec
                .capacity_state()
                .maximum_concurrent_capacity()
                .into(),
        }
    }

    // get
    #[inline]
    pub fn current(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn maximum(&self) -> usize {
        unsafe { *self.maximum_capacity.get() }
    }
}

// HELPERS

impl CapacityState {
    // mut
    #[inline]
    pub(crate) fn set_capacity(&self, new_capacity: usize) {
        debug_assert!(new_capacity > self.current());
        self.capacity.store(new_capacity, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn set_max_capacity(&mut self, new_max_capacity: usize) {
        let maximum_capacity = unsafe { &mut *self.maximum_capacity.get() };
        *maximum_capacity = new_max_capacity;
    }
}

impl Debug for CapacityState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapacityState")
            .field("capacity", &self.capacity)
            .field("maximum_capacity", &self.maximum())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orx_fixed_vec::FixedVec;
    use orx_split_vec::SplitVec;

    #[test]
    fn new_for_pinned_vec() {
        fn validate<P: PinnedVec<String>>(pinned_vec: &P) {
            let capacity = CapacityState::new_for_pinned_vec(pinned_vec);
            assert_eq!(capacity.current(), pinned_vec.capacity());
            assert_eq!(
                capacity.maximum(),
                pinned_vec.capacity_state().maximum_concurrent_capacity()
            );
        }

        validate(&SplitVec::new());
        validate(&SplitVec::with_doubling_growth());
        validate(&SplitVec::with_doubling_growth_and_fragments_capacity(32));
        validate(&SplitVec::with_linear_growth(10));
        validate(&SplitVec::with_linear_growth_and_fragments_capacity(10, 10));
        validate(&SplitVec::with_recursive_growth());
        validate(&SplitVec::with_recursive_growth_and_fragments_capacity(32));
        validate(&FixedVec::new(1024));
    }

    #[test]
    fn set_capacity() {
        let pinned: SplitVec<char> = SplitVec::with_doubling_growth_and_fragments_capacity(4);
        let capacity = CapacityState::new_for_pinned_vec(&pinned);

        assert_eq!(capacity.current(), 4);
        assert_eq!(capacity.maximum(), pinned.maximum_concurrent_capacity());

        capacity.set_capacity(12);

        assert_eq!(capacity.current(), 12);
        assert_eq!(capacity.maximum(), pinned.maximum_concurrent_capacity());
    }

    #[test]
    fn set_max_capacity() {
        let pinned: SplitVec<char, _> = SplitVec::with_linear_growth_and_fragments_capacity(3, 10);
        let mut capacity = CapacityState::new_for_pinned_vec(&pinned);

        assert_eq!(capacity.current(), 8);
        assert_eq!(capacity.maximum(), 8 * 10);

        capacity.set_max_capacity(8 * 15);

        assert_eq!(capacity.current(), 8);
        assert_eq!(capacity.maximum(), 8 * 15);
    }

    #[test]
    fn debug() {
        let pinned: SplitVec<char, _> = SplitVec::with_linear_growth_and_fragments_capacity(3, 10);
        let capacity = CapacityState::new_for_pinned_vec(&pinned);

        let debug = format!("{:?}", capacity);
        assert_eq!(debug, "CapacityState { capacity: 8, maximum_capacity: 80 }");
    }
}
