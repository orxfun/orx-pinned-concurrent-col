use crate::{ConcurrentState, PinnedConcurrentCol};
use orx_fixed_vec::PinnedVec;
use std::fmt::Debug;

impl<T, P, S> Debug for PinnedConcurrentCol<T, P, S>
where
    T: Default,
    P: PinnedVec<T>,
    S: ConcurrentState + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinnedConcurrentCol")
            .field(
                "pinned_vec",
                &self.state().pinned_vec_debug_info(self, self.pinned_vec()),
            )
            .field("state", &self.state())
            .field("capacity", &self.capacity_state())
            .finish()
    }
}
