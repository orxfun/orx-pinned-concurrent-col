mod state;

use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use prelude::FixedVec;
use state::MyConState;

#[test]
fn debug_split_doubling() {
    let mut vec: SplitVec<usize> = SplitVec::with_doubling_growth_and_fragments_capacity(32);
    for i in 0..187 {
        vec.push(i);
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    for i in 187..300 {
        unsafe { col.write(i, i) };
    }

    let debug = format!("{:?}", col);
    let expected = "PinnedConcurrentCol { state: MyConState { initial_len: 187, initial_cap: 252, len: 187 }, capacity: 508, maximum_capacity: 17179869180 }";

    assert_eq!(debug, expected);
}

#[test]
fn debug_split_linear() {
    let mut vec: SplitVec<usize, _> = SplitVec::with_linear_growth_and_fragments_capacity(10, 32);
    for i in 0..187 {
        vec.push(i);
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    for i in 187..1500 {
        unsafe { col.write(i, i) };
    }

    let debug = format!("{:?}", col);
    let expected = "PinnedConcurrentCol { state: MyConState { initial_len: 187, initial_cap: 1024, len: 187 }, capacity: 2048, maximum_capacity: 32768 }";

    assert_eq!(debug, expected);
}

#[test]
fn debug_fixed() {
    let mut vec = FixedVec::new(333);
    for i in 0..187 {
        vec.push(i);
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    for i in 187..300 {
        unsafe { col.write(i, i) };
    }

    let debug = format!("{:?}", col);
    let expected = "PinnedConcurrentCol { state: MyConState { initial_len: 187, initial_cap: 333, len: 187 }, capacity: 333, maximum_capacity: 333 }";

    assert_eq!(debug, expected);
}
