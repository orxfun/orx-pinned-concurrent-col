use crate::{ConcurrentState, PinnedConcurrentCol};
use core::fmt::Debug;
use orx_pinned_vec::ConcurrentPinnedVec;

impl<T, P, S> Debug for PinnedConcurrentCol<T, P, S>
where
    P: ConcurrentPinnedVec<T>,
    S: ConcurrentState<T> + Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PinnedConcurrentCol")
            .field("state", &self.state())
            .field("capacity", &self.capacity())
            .field("maximum_capacity", &self.maximum_capacity())
            .finish()
    }
}
