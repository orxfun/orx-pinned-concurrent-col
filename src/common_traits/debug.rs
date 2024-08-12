use crate::{ConcurrentState, PinnedConcurrentCol};
use orx_pinned_vec::ConcurrentPinnedVec;
use std::fmt::Debug;

impl<T, P, S> Debug for PinnedConcurrentCol<T, P, S>
where
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinnedConcurrentCol")
            .field("state", &self.state())
            .field("capacity", &self.capacity())
            .field("maximum_capacity", &self.maximum_capacity())
            .finish()
    }
}
