mod state;

use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use state::MyConState;

#[test]
fn debug() {
    let mut vec: SplitVec<usize> = SplitVec::new();
    for i in 0..187 {
        vec.push(i);
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    let debug = format!("{:?}", col);
    let expected = "PinnedConcurrentCol { pinned_vec: \"PinnedVec\", state: MyConState { initial_len: 187, initial_cap: 252 }, capacity: CapacityState { capacity: 252, maximum_capacity: 1020 } }";

    assert_eq!(debug, expected);
}
