mod state;
use orx_fixed_vec::FixedVec;
use orx_pinned_concurrent_col::*;
use orx_split_vec::SplitVec;
use prelude::IntoConcurrentPinnedVec;
use state::{MyConState, MyConStateFilled};
use test_case::test_matrix;

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn reserve<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    vec.push("first".to_string());

    let initial_capacity = vec.capacity();

    let mut col: PinnedConcurrentCol<_, _, MyConState<_>> =
        PinnedConcurrentCol::new_from_pinned(vec);
    let max_cap = col.maximum_capacity();

    assert_eq!(col.capacity(), initial_capacity);

    let new_capacity = unsafe { col.reserve_maximum_capacity(1, max_cap + 1) };

    assert!(new_capacity >= max_cap + 1);
    assert!(col.capacity() >= initial_capacity);
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn reserve_fill_with<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    vec.push("first".to_string());

    let initial_capacity = vec.capacity();

    let mut col: PinnedConcurrentCol<_, _, MyConStateFilled<_>> =
        PinnedConcurrentCol::new_from_pinned(vec);
    let max_cap = col.maximum_capacity();

    assert_eq!(col.capacity(), initial_capacity);

    let new_capacity = unsafe { col.reserve_maximum_capacity(1, max_cap + 1) };

    assert!(new_capacity >= max_cap + 1);
    assert!(col.capacity() >= initial_capacity);
}
