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
fn new_no_fill<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    vec.push("first".to_string());

    let initial_len = vec.len();
    let initial_capacity = vec.capacity();

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    let vec = unsafe { col.into_inner(initial_len) };

    assert_eq!(vec.len(), 1);
    assert_eq!(vec.capacity(), initial_capacity);
    assert_eq!(&vec[0], &"first".to_string());
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn new_fill_with<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    vec.push("first".to_string());

    let initial_capacity = vec.capacity();

    let col: PinnedConcurrentCol<_, _, MyConStateFilled<_>> =
        PinnedConcurrentCol::new_from_pinned(vec);

    let vec = unsafe { col.into_inner(initial_capacity) };

    assert_eq!(vec.len(), initial_capacity);
    assert_eq!(vec.capacity(), initial_capacity);
    assert_eq!(&vec[0], &"first".to_string());
    assert_eq!(
        vec.into_iter().skip(1).collect::<Vec<_>>(),
        (1..initial_capacity)
            .map(|_| String::default())
            .collect::<Vec<_>>()
    );
}
