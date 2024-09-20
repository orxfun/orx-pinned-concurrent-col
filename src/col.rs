use crate::{
    errors::*, mem_state::VecDropState, state::ConcurrentState, write_permit::WritePermit,
};
use core::{marker::PhantomData, ops::RangeBounds};
use orx_pinned_vec::{ConcurrentPinnedVec, IntoConcurrentPinnedVec, PinnedVec};
use orx_pseudo_default::PseudoDefault;

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
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T>,
{
    phantom: PhantomData<T>,
    con_pinned_vec: P,
    state: S,
    vec_drop_state: VecDropState,
}

impl<T, P, S> Drop for PinnedConcurrentCol<T, P, S>
where
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T>,
{
    fn drop(&mut self) {
        match self.vec_drop_state {
            VecDropState::ToBeDropped => {
                let len = match self.state().fill_memory_with().is_some() {
                    true => self.con_pinned_vec.capacity(),
                    false => {
                        let capacity = self.con_pinned_vec.capacity();
                        let no_gap_len = self.state.try_get_no_gap_len().unwrap_or(capacity);
                        [no_gap_len].into_iter().fold(capacity, usize::min)
                    }
                };
                self.vec_drop_state = VecDropState::TakenOut;
                unsafe { self.con_pinned_vec.set_pinned_vec_len(len) };
            }
            VecDropState::TakenOut => {
                let len = match self.state().fill_memory_with().is_some() {
                    true => self.con_pinned_vec.capacity(),
                    false => 0,
                };
                unsafe { self.con_pinned_vec.set_pinned_vec_len(len) };
            }
        }
    }
}

impl<T, P, S> PinnedConcurrentCol<T, P, S>
where
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T>,
{
    // new
    /// Wraps the `pinned_vec` and converts it into a pinned concurrent collection.
    pub fn new_from_pinned<Q>(pinned_vec: Q) -> Self
    where
        Q: IntoConcurrentPinnedVec<T, ConPinnedVec = P>,
    {
        let state = S::new_for_pinned_vec(&pinned_vec);

        let con_pinned_vec = match state.fill_memory_with() {
            None => pinned_vec.into_concurrent(),
            Some(f) => pinned_vec.into_concurrent_filled_with(f),
        };

        Self {
            phantom: Default::default(),
            state,
            con_pinned_vec,
            vec_drop_state: VecDropState::ToBeDropped,
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
    pub unsafe fn into_inner(mut self, pinned_vec_len: usize) -> P::P
    where
        P::P: IntoConcurrentPinnedVec<T, ConPinnedVec = P>,
    {
        self.vec_drop_state = VecDropState::TakenOut;

        let mut inner = <P::P as PseudoDefault>::pseudo_default().into_concurrent();
        match self.state.fill_memory_with() {
            Some(fill_with) => {
                inner.fill_with(0..inner.capacity(), fill_with);
                inner.set_pinned_vec_len(inner.capacity());
            }
            None => inner.set_pinned_vec_len(0),
        }

        core::mem::swap(&mut inner, &mut self.con_pinned_vec);

        inner.into_inner(pinned_vec_len)
    }

    /// Clones the underlying pinned vector, sets its length to the given `pinned_vec_len` and returns the vector.
    ///
    /// # Safety
    ///
    /// This method is unsafe as pinned concurrent collection does not keep track of the writes and length.
    /// This is the responsibility of the wrapper through the specific `ConcurrentState` implementation.
    /// Therefore, the following situation is possible:
    /// * concurrent collection is created with an empty pinned vector.
    /// * the caller calls `reserve_maximum_capacity` with sufficient capacity, say 2.
    /// * then, `write(1, value)` is called by writing to the second position, skipping the first position.
    /// * and finally, calls `clone_inner(2)`.
    ///
    /// This would return a pinned vector with a valid entry at position 1 but uninitialized value at position 0, which would lead to an undefined behavior.
    ///
    /// Therefore, the wrapper must ensure that the pinned vector is in a valid state before taking it out.
    ///
    /// ## Safe Usage Examples
    ///
    /// The unsafe `clone_inner` method can be wrapped with a safe method if the following guarantee is satisfied:
    /// * All values in range `0..pinned_vec_len` of the concurrent collection are written.
    ///
    /// Two such example wrappers are [`ConcurrentBag`](https://crates.io/crates/orx-concurrent-bag) and [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent bag and vector do not allow leaving gaps, and only push to the back of the collection.
    /// - Furthermore, they keep track of the number of pushes.
    /// - Therefore, they can safely extract the pinned vector out with the length that it correctly knows.
    pub unsafe fn clone_with_len(&self, pinned_vec_len: usize) -> Self
    where
        T: Clone,
    {
        let con_pinned_vec = self.con_pinned_vec.clone_with_len(pinned_vec_len);
        if let Some(fill_with) = self.state.fill_memory_with() {
            let range_to_fill = pinned_vec_len..con_pinned_vec.capacity();
            con_pinned_vec.fill_with(range_to_fill, fill_with);
        }

        let state = S::new_for_con_pinned_vec(&con_pinned_vec, pinned_vec_len);
        Self {
            phantom: Default::default(),
            state,
            con_pinned_vec,
            vec_drop_state: VecDropState::ToBeDropped,
        }
    }

    // getters

    /// Returns a reference to the current concurrent state of the collection.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Returns the current allocated capacity of the collection.
    pub fn capacity(&self) -> usize {
        self.con_pinned_vec.capacity()
    }

    /// Returns maximum possible capacity that the collection can reach without calling [`PinnedConcurrentCol::reserve_maximum_capacity`].
    ///
    /// Importantly note that maximum capacity does not correspond to the allocated memory.
    pub fn maximum_capacity(&self) -> usize {
        self.con_pinned_vec.max_capacity()
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
        self.con_pinned_vec.iter(len)
    }

    /// Returns an iterator to the elements of the underlying pinned vector over the given `range`.
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
    ///* All values in `range` of the concurrent collection are written.
    ///
    /// An example can be seen in [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent vec zeroes memory on allocation.
    /// - Furthermore, it uses a pinned vector of `Option<T>` to represent a collection of `T`s. It has a valid zero value, `Option::None`.
    /// - The iter wrapper simply skips `None`s which correspond to uninitialized values.
    pub unsafe fn iter_over_range<R: RangeBounds<usize>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = &T> {
        self.con_pinned_vec.iter_over_range(range)
    }

    /// Returns a mutable iterator to the elements of the underlying pinned vector starting from the first element and taking `len` elements.
    ///
    /// # Safety
    ///
    /// This method is unsafe due to the following reasons:
    ///
    /// * `PinnedConcurrentCol` does not guarantee that all positions are initialized.
    /// It is possible to create the collection, skip the first position and directly write to the second position.
    /// In this case, the `iter` call would read an uninitialized value at the first position.
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
    pub unsafe fn iter_mut(&mut self, len: usize) -> impl Iterator<Item = &mut T> {
        self.con_pinned_vec.iter_mut(len)
    }

    /// Returns a reference to the element written at the `index`-th position.
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
        self.con_pinned_vec.get(index)
    }

    /// Returns a mutable reference to the element written at the `index`-th position.
    ///
    /// # Safety
    ///
    /// This method is unsafe due to the following reason.
    ///
    /// * `PinnedConcurrentCol` does not guarantee that all positions are initialized.
    /// It is possible to create the collection, skip the first position and directly write to the second position.
    /// In this case, the `get` call would read an uninitialized value at the first position.
    ///
    /// ## Example Safe Usage
    ///
    /// This method can be wrapped by a safe method provided that the following safety requirement can be guaranteed:
    /// * The value at position `index` is written.
    ///
    /// An example can be seen in [`ConcurrentVec`](https://crates.io/crates/orx-concurrent-vec).
    /// - Concurrent vec zeroes memory on allocation.
    /// - Furthermore, it uses a pinned vector of `Option<T>` to represent a collection of `T`s. It has a valid zero value, `Option::None`.
    /// - The get_mut method wrapper will return `None` for uninitialized values.
    pub unsafe fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.con_pinned_vec.get_mut(index)
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
    ///
    /// # Safety
    /// This method is unsafe since the concurrent pinned vector might contain gaps. The vector must be gap-free while increasing the maximum capacity.
    ///
    /// This method can safely be called if entries in all positions 0..len are written.
    pub unsafe fn reserve_maximum_capacity(
        &mut self,
        current_len: usize,
        maximum_capacity: usize,
    ) -> usize {
        match self.state.fill_memory_with() {
            Some(fill_with) => self
                .con_pinned_vec
                .reserve_maximum_concurrent_capacity_fill_with(
                    current_len,
                    maximum_capacity,
                    fill_with,
                ),
            None => self
                .con_pinned_vec
                .reserve_maximum_concurrent_capacity(current_len, maximum_capacity),
        }
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

    /// Reserves and returns a reference for one position at the `idx`-th position.
    ///
    /// The caller is responsible for writing to the position.
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
    /// Furthermore, the caller is responsible to write all positions of the acquired slices to make sure that the collection is gap free.
    ///
    /// Note that although both methods are unsafe, it is much easier to achieve required safety guarantees with `write_n_items`;
    /// hence, it must be preferred unless there is a good reason to acquire mutable slices.
    /// One such example case is to copy results directly into the output's slices, which could be more performant in a very critical scenario.
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn single_item_as_ref(&self, idx: usize) -> &T {
        self.assert_has_capacity_for(idx);
        loop {
            let write_permit = self.state.write_permit(self, idx);
            match write_permit {
                WritePermit::JustWrite => {
                    let x = self
                        .con_pinned_vec
                        .get(idx)
                        .expect("should succeed since has capacity for idx");
                    self.state.update_after_write(idx, idx + 1);
                    return x;
                }
                WritePermit::GrowThenWrite => {
                    self.grow_to(idx + 1);
                    self.state.update_after_write(idx, idx + 1);
                    let x = self
                        .con_pinned_vec
                        .get(idx)
                        .expect("should succeed since has capacity for idx");
                    return x;
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
                        self.write_n_items_at(begin_idx, num_items, values);
                        self.state.update_after_write(begin_idx, end_idx);
                        break;
                    }
                    WritePermit::GrowThenWrite => {
                        self.grow_to(end_idx);
                        self.write_n_items_at(begin_idx, num_items, values);
                        self.state.update_after_write(begin_idx, end_idx);
                        break;
                    }
                    WritePermit::Spin => {}
                }
            }
        }
    }

    /// Reserves and returns an iterator of mutable slices for `num_items` positions starting from the `begin_idx`-th position.
    ///
    /// The caller is responsible for filling all `num_items` positions in the returned iterator of slices with values to avoid gaps.
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
    /// Furthermore, the caller is responsible to write all positions of the acquired slices to make sure that the collection is gap free.
    ///
    /// Note that although both methods are unsafe, it is much easier to achieve required safety guarantees with `write_n_items`;
    /// hence, it must be preferred unless there is a good reason to acquire mutable slices.
    /// One such example case is to copy results directly into the output's slices, which could be more performant in a very critical scenario.
    pub unsafe fn n_items_buffer_as_slices(
        &self,
        begin_idx: usize,
        num_items: usize,
    ) -> <P::P as PinnedVec<T>>::SliceIter<'_> {
        match num_items {
            0 => <P::P as PinnedVec<T>>::SliceIter::default(),
            _ => {
                let end_idx = begin_idx + num_items;
                let last_idx = end_idx - 1;
                self.assert_has_capacity_for(last_idx);

                loop {
                    match self.state.write_permit_n_items(self, begin_idx, num_items) {
                        WritePermit::JustWrite => {
                            let slices = self.slices_for_n_items_at(begin_idx, num_items);
                            self.state.update_after_write(begin_idx, end_idx);
                            return slices;
                        }
                        WritePermit::GrowThenWrite => {
                            self.grow_to(end_idx);
                            let slices = self.slices_for_n_items_at(begin_idx, num_items);
                            self.state.update_after_write(begin_idx, end_idx);
                            return slices;
                        }
                        WritePermit::Spin => {}
                    }
                }
            }
        }
    }

    /// Reserves and returns an iterator of mutable slices for `num_items` positions starting from the `begin_idx`-th position.
    ///
    /// The caller is responsible for filling all `num_items` positions in the returned iterator of slices with values to avoid gaps.
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
    /// Furthermore, the caller is responsible to write all positions of the acquired slices to make sure that the collection is gap free.
    ///
    /// Note that although both methods are unsafe, it is much easier to achieve required safety guarantees with `write_n_items`;
    /// hence, it must be preferred unless there is a good reason to acquire mutable slices.
    /// One such example case is to copy results directly into the output's slices, which could be more performant in a very critical scenario.
    pub unsafe fn n_items_buffer_as_mut_slices(
        &self,
        begin_idx: usize,
        num_items: usize,
    ) -> <P::P as PinnedVec<T>>::SliceMutIter<'_> {
        match num_items {
            0 => <P::P as PinnedVec<T>>::SliceMutIter::default(),
            _ => {
                let end_idx = begin_idx + num_items;
                let last_idx = end_idx - 1;
                self.assert_has_capacity_for(last_idx);

                loop {
                    match self.state.write_permit_n_items(self, begin_idx, num_items) {
                        WritePermit::JustWrite => {
                            let slices = self.slices_mut_for_n_items_at(begin_idx, num_items);
                            self.state.update_after_write(begin_idx, end_idx);
                            return slices;
                        }
                        WritePermit::GrowThenWrite => {
                            self.grow_to(end_idx);
                            let slices = self.slices_mut_for_n_items_at(begin_idx, num_items);
                            self.state.update_after_write(begin_idx, end_idx);
                            return slices;
                        }
                        WritePermit::Spin => {}
                    }
                }
            }
        }
    }

    /// Clears the collection.
    ///
    /// # Safety
    /// This method is unsafe since the concurrent pinned vector might contain gaps.
    ///
    /// This method can safely be called if entries in all positions 0..len are written
    pub unsafe fn clear(&mut self, prior_len: usize) {
        self.con_pinned_vec.clear(prior_len);
        self.state = S::new_for_con_pinned_vec(&self.con_pinned_vec, 0);
    }
}

// HELPERS

impl<T, P, S> PinnedConcurrentCol<T, P, S>
where
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T>,
{
    #[inline]
    fn assert_has_capacity_for(&self, idx: usize) {
        assert!(
            idx < self.con_pinned_vec.max_capacity(),
            "{}",
            ERR_REACHED_MAX_CAPACITY
        );
    }

    #[inline]
    fn write_at(&self, idx: usize, value: T) {
        let ptr = unsafe { self.con_pinned_vec.get_ptr_mut(idx) };
        unsafe { ptr.write(value) };
    }

    fn write_n_items_at<I>(&self, begin_idx: usize, num_items: usize, values: I)
    where
        I: IntoIterator<Item = T>,
    {
        const ERR_SHORT_ITER: &str = "iterator is shorter than expected num_items";

        let mut values = values.into_iter();

        let slices = self.slices_mut_for_n_items_at(begin_idx, num_items);
        for slice in slices {
            let ptr = slice.as_mut_ptr();
            let len = slice.len();
            for i in 0..len {
                unsafe { ptr.add(i).write(values.next().expect(ERR_SHORT_ITER)) };
            }
        }
    }

    #[inline]
    fn slices_mut_for_n_items_at(
        &self,
        begin_idx: usize,
        num_items: usize,
    ) -> <P::P as PinnedVec<T>>::SliceMutIter<'_> {
        let end_idx = begin_idx + num_items;
        unsafe { self.con_pinned_vec.slices_mut(begin_idx..end_idx) }
    }

    #[inline]
    fn slices_for_n_items_at(
        &self,
        begin_idx: usize,
        num_items: usize,
    ) -> <P::P as PinnedVec<T>>::SliceIter<'_> {
        let end_idx = begin_idx + num_items;
        self.con_pinned_vec.slices(begin_idx..end_idx)
    }

    fn grow_to(&self, new_capacity: usize) {
        match self.state.fill_memory_with() {
            None => {
                let _new_capacity = self
                    .con_pinned_vec
                    .grow_to(new_capacity)
                    .expect(ERR_FAILED_TO_GROW);
            }
            Some(f) => {
                let _new_capacity = self
                    .con_pinned_vec
                    .grow_to_and_fill_with(new_capacity, f)
                    .expect(ERR_FAILED_TO_GROW);
            }
        }

        self.state.release_growth_handle();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        cmp::Ordering,
        sync::atomic::{self, AtomicUsize},
    };
    use orx_pinned_vec::PinnedVec;

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct MyConState<T> {
        pub initial_len: usize,
        pub initial_cap: usize,
        pub len: AtomicUsize,
        phantom: PhantomData<T>,
    }

    impl<T> MyConState<T> {
        pub fn new(initial_len: usize, initial_cap: usize) -> Self {
            Self {
                initial_len,
                initial_cap,
                len: initial_len.into(),
                phantom: Default::default(),
            }
        }

        #[inline(always)]
        pub(crate) fn len(&self) -> usize {
            self.len.load(atomic::Ordering::SeqCst)
        }
    }

    impl<T> ConcurrentState<T> for MyConState<T> {
        fn fill_memory_with(&self) -> Option<fn() -> T> {
            None
        }

        fn new_for_pinned_vec<P: PinnedVec<T>>(pinned_vec: &P) -> Self {
            Self::new(pinned_vec.len(), pinned_vec.capacity())
        }

        fn new_for_con_pinned_vec<P: ConcurrentPinnedVec<T>>(
            con_pinned_vec: &P,
            len: usize,
        ) -> Self {
            Self::new(len, con_pinned_vec.capacity())
        }

        fn write_permit<P>(&self, col: &PinnedConcurrentCol<T, P, Self>, idx: usize) -> WritePermit
        where
            P: ConcurrentPinnedVec<T>,
        {
            let capacity = col.capacity();

            match idx.cmp(&capacity) {
                Ordering::Less => WritePermit::JustWrite,
                Ordering::Equal => WritePermit::GrowThenWrite,
                Ordering::Greater => WritePermit::Spin,
            }
        }

        fn write_permit_n_items<P>(
            &self,
            col: &PinnedConcurrentCol<T, P, Self>,
            begin_idx: usize,
            num_items: usize,
        ) -> WritePermit
        where
            P: ConcurrentPinnedVec<T>,
        {
            let capacity = col.capacity();
            let last_idx = begin_idx + num_items - 1;

            match (begin_idx.cmp(&capacity), last_idx.cmp(&capacity)) {
                (_, core::cmp::Ordering::Less) => WritePermit::JustWrite,
                (core::cmp::Ordering::Greater, _) => WritePermit::Spin,
                _ => WritePermit::GrowThenWrite,
            }
        }

        fn release_growth_handle(&self) {}

        fn update_after_write(&self, _: usize, _: usize) {}

        fn try_get_no_gap_len(&self) -> Option<usize> {
            Some(self.len())
        }
    }
}
