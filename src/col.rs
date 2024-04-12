use crate::{
    capacity::CapacityState, errors::*, state::ConcurrentState, write_permit::WritePermit,
};
use orx_pinned_vec::PinnedVec;
use std::{cell::UnsafeCell, marker::PhantomData};

/// A core data structure with a focus to enable high performance, possibly lock-free, concurrent collections using a [`PinnedVec`](https://crates.io/crates/orx-pinned-vec) as the underlying storage.
///
/// Pinned vectors grow while keeping the already pushed elements pinned to their memory locations. This allows the following concurrency model.
///
/// * Writing to the collection does not block. Multiple writes can happen concurrently.
///   * However, `PinnedConcurrentCol` itself does not provide guarantees for race-free writing; and hence, the write methods are marked `unsafe`.
///   * It is the responsibility of the wrapper to make sure that multiple writes or reading during write to the same position do not happen concurrently.
/// * Only one growth (capacity expansion) can happen at a given time.
///   * If the underlying collection reaches its capacity and needs to grow, one and only one thread takes the responsibility to expand the vector.
/// * Growth does not block.
///   * Writes to positions which are already within capacity are not blocked by the growth.
///   * Writes to to-be-allocated positions wait only for the allocation to be completed; not any other task of the thread responsible for expansion.
///
/// As clear from the properties, pinned concurrent collection aims to achieve high performance. It exposes the useful methods that can be used differently for different requirements and marks the methods which can lead to race conditions as `unsafe` by stating the underlying reasons. This enables building safe wrappers such as [`ConcurrentBag`](https://crates.io/crates/orx-concurrent-bag), [`ConcurrentOrderedBag`](https://crates.io/crates/orx-concurrent-ordered-bag) or [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
pub struct PinnedConcurrentCol<T, P, S>
where
    P: PinnedVec<T>,
    S: ConcurrentState,
{
    phantom: PhantomData<T>,
    pinned_vec: UnsafeCell<P>,
    state: S,
    capacity: CapacityState,
}

impl<T, P, S> PinnedConcurrentCol<T, P, S>
where
    P: PinnedVec<T>,
    S: ConcurrentState,
{
    // new
    /// Wraps the `pinned_vec` and converts it into a pinned concurrent collection.
    pub fn new_from_pinned(pinned_vec: P) -> Self {
        let state = S::new_for_pinned_vec(&pinned_vec);
        let capacity = CapacityState::new_for_pinned_vec(&pinned_vec);

        let mut pinned_vec = pinned_vec;
        unsafe { pinned_vec.set_len(0) };

        Self {
            phantom: Default::default(),
            state,
            capacity,
            pinned_vec: pinned_vec.into(),
        }
    }

    // into
    /// Sets the length of the underlying pinned vector to the given `pinned_vec_len` and returns the vector.
    ///
    /// # Safety
    ///
    /// This method is unsafe as pinned concurrent collection does not keep track of the writes and length.
    /// This is the responsibility of the wrapper through the specific `ConcurrentState` implementation.
    /// Therefore, the following situation is possible:
    /// * concurrent collection is created with an empty pinned vector.
    /// * the caller calls `reserve_maximum_capacity` with sufficient capacity, say 2.
    /// * then, `write(1, value)` is called by writing to the second position, skipping the first position.
    /// * and finally, calls `into_inner(2)`.
    ///
    /// This would return a pinned vector with a valid entry at position 1 but uninitialized value at position 0, which would lead to an undefined behavior.
    ///
    /// Therefore, the wrapper must ensure that the pinned vector is in a valid state before taking it out.
    ///
    /// ## Safe Usage Examples
    ///
    /// The unsafe `into_inner` method can be wrapped with a safe method if the following guarantee is satisfied:
    /// * All values in range `0..pinned_vec_len` of the concurrent collection are written.
    ///
    /// Two such example wrappers are [`ConcurrentBag`](https://crates.io/crates/orx-concurrent-bag) and [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent bag and vector do not allow leaving gaps, and only push to the back of the collection.
    /// - Furthermore, they keep track of the number of pushes.
    /// - Therefore, they can safely extract the pinned vector out with the length that it correctly knows.
    pub unsafe fn into_inner(self, pinned_vec_len: usize) -> P {
        unsafe { self.set_pinned_vec_len(pinned_vec_len) };
        self.pinned_vec.into_inner()
    }

    // getters
    /// Returns a reference to the current concurrent state of the collection.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Returns the current allocated capacity of the collection.
    pub fn capacity(&self) -> usize {
        self.capacity.current()
    }

    /// Returns maximum possible capacity that the collection can reach without calling [`PinnedConcurrentCol::reserve_maximum_capacity`].
    ///
    /// Importantly note that maximum capacity does not correspond to the allocated memory.
    pub fn maximum_capacity(&self) -> usize {
        self.capacity.maximum()
    }

    /// Returns whether or not the collection zeroes out memory on allocation.
    /// Note that this is determined by [`ConcurrentState::zero_memory`] method of the underlying state.
    #[inline]
    pub fn zeroes_memory_on_allocation(&self) -> bool {
        S::zero_memory()
    }

    // unsafe getters
    /// Returns an iterator to the elements of the underlying pinned vector starting from the first element and taking `len` elements.
    ///
    /// # Safety
    ///
    /// This method is unsafe due to two reasons.
    ///
    /// * Firstly, `PinnedConcurrentCol` does not guarantee that all positions are initialized.
    /// It is possible to create the collection, skip the first position and directly write to the second position.
    /// In this case, the `iter` call would read an uninitialized value at the first position.
    ///
    /// * Secondly, `PinnedConcurrentCol` focuses on lock-free writing.
    /// Therefore, while the iterator is reading an element, another thread might be writing to this position.
    ///
    /// ## Example Safe Usage
    ///
    /// This method can be wrapped by a safe method provided that the following safety requirement can be guaranteed:
    ///* All values in range `0..pinned_vec_len` of the concurrent collection are written.
    ///
    /// An example can be seen in [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent vec zeroes memory on allocation.
    /// - Furthermore, it uses a pinned vector of `Option<T>` to represent a collection of `T`s. It has a valid zero value, `Option::None`.
    /// - The iter wrapper simply skips `None`s which correspond to uninitialized values.
    pub unsafe fn iter(&self, len: usize) -> impl Iterator<Item = &T> {
        let pinned = unsafe { &mut *self.pinned_vec.get() };
        unsafe { pinned.set_len(len) };
        let iter = pinned.iter().take(len);
        iter
    }

    /// Returns the element written at the `index`-th position.
    ///
    /// # Safety
    ///
    /// This method is unsafe due to two reasons.
    ///
    /// * Firstly, `PinnedConcurrentCol` does not guarantee that all positions are initialized.
    /// It is possible to create the collection, skip the first position and directly write to the second position.
    /// In this case, the `get` call would read an uninitialized value at the first position.
    ///
    /// * Secondly, `PinnedConcurrentCol` focuses on lock-free writing.
    /// Therefore, while the get method is reading an element, another thread might be writing to this position.
    ///
    /// ## Example Safe Usage
    ///
    /// This method can be wrapped by a safe method provided that the following safety requirement can be guaranteed:
    /// * The value at position `index` is written.
    ///
    /// An example can be seen in [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent vec zeroes memory on allocation.
    /// - Furthermore, it uses a pinned vector of `Option<T>` to represent a collection of `T`s. It has a valid zero value, `Option::None`.
    /// - The get method wrapper simply the value, which will be `None` for uninitialized values.
    pub unsafe fn get(&self, index: usize) -> Option<&T> {
        if index < self.capacity() {
            let pinned = unsafe { &mut *self.pinned_vec.get() };
            let ptr = unsafe { pinned.get_ptr_mut(index) };
            ptr.and_then(|x| x.as_ref())
        } else {
            None
        }
    }

    // mutations
    /// Note that [`PinnedConcurrentCol::maximum_capacity`] returns the maximum possible number of elements that the underlying pinned vector can grow to without reserving maximum capacity.
    ///
    /// In other words, the pinned vector can automatically grow up to the [`PinnedConcurrentCol::maximum_capacity`] with `write` and `write_n_items` methods, using only a shared reference.
    ///
    /// When required, this maximum capacity can be attempted to increase by this method with a mutable reference.
    ///
    /// Importantly note that maximum capacity does not correspond to the allocated memory.
    ///
    /// Among the common pinned vector implementations:
    /// * `SplitVec<_, Doubling>`: supports this method; however, it does not require for any practical size.
    /// * `SplitVec<_, Linear>`: is guaranteed to succeed and increase its maximum capacity to the required value.
    /// * `FixedVec<_>`: is the most strict pinned vector which cannot grow even in a single-threaded setting. Currently, it will always return an error to this call.
    pub fn reserve_maximum_capacity(&mut self, maximum_capacity: usize) -> Result<usize, String> {
        let pinned = unsafe { &mut *self.pinned_vec.get() };
        pinned
            .try_reserve_maximum_concurrent_capacity(maximum_capacity)
            .map(|x| {
                self.capacity.set_max_capacity(x);
                x
            })
    }

    /// Writes the `value` to the `idx`-th position.
    ///
    /// # Safety
    ///
    /// This method makes sure that the value is written to a position owned by the underlying pinned vector.
    /// Furthermore, it makes sure that the growth of the vector happens thread-safely whenever necessary.
    ///
    /// On the other hand, it is unsafe due to the possibility of a race condition.
    /// Multiple threads can try to write to the same `idx` at the same time.
    /// The wrapper is responsible for preventing this.
    ///
    /// This method can safely be used provided that the caller provides the following guarantee:
    /// * **multiple `write` or `write_n_items` calls which writes to the same `idx` must not happen concurrently.**
    pub unsafe fn write(&self, idx: usize, value: T) {
        self.assert_has_capacity_for(idx);

        loop {
            let write_permit = self.state.write_permit(self, idx);
            match write_permit {
                WritePermit::JustWrite => {
                    self.write_at(idx, value);
                    self.state.update_after_write(idx, idx + 1);
                    break;
                }
                WritePermit::GrowThenWrite => {
                    self.grow_to(idx + 1);
                    self.write_at(idx, value);
                    self.state.update_after_write(idx, idx + 1);
                    break;
                }
                WritePermit::Spin => {}
            }
        }
    }

    /// Writes the `num_items` `values` to sequential positions starting from the `begin_idx`-th position.
    ///
    /// * If the `values` iterator has more than `num_items` elements, the excess values will be ignored.
    /// * The method will not complain; however, `values` iterator yielding less than `num_items` elements might lead to safety issues (see below).
    ///
    ///
    /// # Safety
    ///
    /// This method makes sure that the values are written to positions owned by the underlying pinned vector.
    /// Furthermore, it makes sure that the growth of the vector happens thread-safely whenever necessary.
    ///
    /// On the other hand, it is unsafe due to the possibility of a race condition.
    /// Multiple threads can try to write to the same position at the same time.
    /// The wrapper is responsible for preventing this.
    ///
    /// This method can safely be used provided that the caller provides the following guarantees:
    /// * **multiple `write` or `write_n_items` calls which writes to the same `idx` must not happen concurrently.**
    /// * **values** iterator yielding less than `num_items` elements might lead to gaps in the bag, which would lead to gaps in the vector if not handled properly.
    pub unsafe fn write_n_items<IntoIter>(
        &self,
        begin_idx: usize,
        num_items: usize,
        values: IntoIter,
    ) where
        IntoIter: IntoIterator<Item = T>,
    {
        if num_items > 0 {
            let end_idx = begin_idx + num_items;
            let last_idx = end_idx - 1;
            self.assert_has_capacity_for(last_idx);

            loop {
                match self.state.write_permit_n_items(self, begin_idx, num_items) {
                    WritePermit::JustWrite => {
                        for (i, value) in values.into_iter().enumerate() {
                            self.write_at(begin_idx + i, value);
                        }
                        self.state.update_after_write(begin_idx, end_idx);
                        break;
                    }
                    WritePermit::GrowThenWrite => {
                        self.grow_to(end_idx);
                        for (i, value) in values.into_iter().take(num_items).enumerate() {
                            self.write_at(begin_idx + i, value);
                        }
                        self.state.update_after_write(begin_idx, end_idx);
                        break;
                    }
                    WritePermit::Spin => {}
                }
            }
        }
    }

    /// Clears the collection.
    pub fn clear(&mut self) {
        let pinned = unsafe { &mut *self.pinned_vec.get() };
        pinned.clear();

        self.capacity = CapacityState::new_for_pinned_vec(pinned);
        self.state = S::new_for_pinned_vec(pinned);
    }
}

// HELPERS

impl<T, P, S> PinnedConcurrentCol<T, P, S>
where
    P: PinnedVec<T>,
    S: ConcurrentState,
{
    #[inline]
    fn assert_has_capacity_for(&self, idx: usize) {
        assert!(
            idx < self.capacity.maximum(),
            "{}",
            ERR_REACHED_MAX_CAPACITY
        );
    }

    #[inline]
    fn write_at(&self, idx: usize, value: T) {
        let pinned = unsafe { &mut *self.pinned_vec.get() };
        let ptr = unsafe { pinned.get_ptr_mut(idx) }.expect(ERR_FAILED_TO_PUSH);
        unsafe { std::ptr::write(ptr, value) };
    }

    fn grow_to(&self, new_capacity: usize) {
        let pinned = unsafe { &mut *self.pinned_vec.get() };

        let new_capacity =
            unsafe { pinned.grow_to(new_capacity, self.zeroes_memory_on_allocation()) }
                .expect(ERR_FAILED_TO_GROW);

        self.capacity.set_capacity(new_capacity);
        self.state.release_growth_handle();
    }

    /// Sets the length of the underlying pinned vector to the given `pinned_vec_len`.
    ///
    /// # Panics
    ///
    /// Panics if `pinned_vec_len > self.capacity()`.
    ///
    /// # Safety
    ///
    /// `PinnedConcurrentCol` allows and does not track gaps in the underlying vector.
    /// Therefore, setting its length to a value including uninitialized values might lead to safety issues.
    /// * It is partially okay since the collection does not expose the pinned vector. In other words, one cannot access the uninitialized memory, even if it exists.
    /// * However, if the underlying element type does not have valid uninitialized value, this might still lead to a memory corruption.
    unsafe fn set_pinned_vec_len(&self, pinned_vec_len: usize) {
        let pinned = unsafe { &mut *self.pinned_vec.get() };
        assert!(pinned_vec_len <= pinned.capacity());
        unsafe { pinned.set_len(pinned_vec_len) };
    }

    pub(crate) fn pinned_vec(&self) -> &P {
        unsafe { &*self.pinned_vec.get() }
    }

    pub(crate) fn capacity_state(&self) -> &CapacityState {
        &self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orx_pinned_vec::PinnedVec;
    use orx_split_vec::SplitVec;
    use std::cmp::Ordering;
    use test_case::test_matrix;

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
        fn zero_memory() -> bool {
            false
        }

        fn new_for_pinned_vec<T, P: PinnedVec<T>>(pinned_vec: &P) -> Self {
            Self::new(pinned_vec.len(), pinned_vec.capacity())
        }

        fn write_permit<T, P, S>(
            &self,
            col: &PinnedConcurrentCol<T, P, S>,
            idx: usize,
        ) -> WritePermit
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
    }

    #[test]
    fn set_pinned_vec_len_in_len() {
        for _ in 0..140 {
            let mut vec: SplitVec<String> =
                SplitVec::with_doubling_growth_and_fragments_capacity(32);
            vec.push("a".to_string());
            vec.push("b".to_string());
            vec.push("c".to_string());
            vec.push("d".to_string());
            vec.push("e".to_string());
            vec.push("f".to_string());

            let col: PinnedConcurrentCol<_, _, MyConState> =
                PinnedConcurrentCol::new_from_pinned(vec);

            for ok_len in 0..6 {
                unsafe { col.set_pinned_vec_len(ok_len) };
            }
        }
    }

    #[test_matrix([7, 8, 9, 10, 11, 12])]
    fn set_pinned_vec_len_in_capacity_with_stack(within_capacity_len: usize) {
        for _ in 0..140 {
            let mut vec: SplitVec<&'static str> =
                SplitVec::with_doubling_growth_and_fragments_capacity(32);
            vec.push("a");
            vec.push("b");
            vec.push("c");
            vec.push("d");
            vec.push("e");
            vec.push("f");

            let col: PinnedConcurrentCol<_, _, MyConState> =
                PinnedConcurrentCol::new_from_pinned(vec);
            unsafe { col.set_pinned_vec_len(within_capacity_len) };
        }
    }

    #[test_matrix([13, 14, 15, 16, 17, 18])]
    #[should_panic]
    fn set_pinned_vec_len_out_of_capacity(out_of_capacity_len: usize) {
        let mut vec: SplitVec<usize> = SplitVec::with_doubling_growth_and_fragments_capacity(32);
        for i in 0..6 {
            vec.push(i);
        }

        let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);
        unsafe { col.set_pinned_vec_len(out_of_capacity_len) };
    }
}
