mod state;

use orx_fixed_vec::FixedVec;
use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use prelude::IntoConcurrentPinnedVec;
use state::MyConState;
use test_case::test_matrix;

#[test]
fn new_from_pinned() {
    let pinned_vec: SplitVec<String> = SplitVec::with_doubling_growth_and_fragments_capacity(32);
    let expected_state = MyConState::new_for_pinned_vec(&pinned_vec);

    let col: PinnedConcurrentCol<_, _, MyConState<_>> =
        PinnedConcurrentCol::new_from_pinned(pinned_vec.clone());

    assert_eq!(col.state().initial_len, expected_state.initial_len);
    assert_eq!(col.state().initial_cap, expected_state.initial_cap);
    assert_eq!(col.capacity(), pinned_vec.capacity());
    assert_eq!(
        col.maximum_capacity(),
        pinned_vec.capacity_state().maximum_concurrent_capacity()
    );
}

#[test]
fn into_inner() {
    let mut vec: SplitVec<String> = SplitVec::new();
    vec.push("a".to_string());
    vec.push("b".to_string());

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    let vec_back = unsafe { col.into_inner(2) };
    assert_eq!(&vec_back, &["a".to_string(), "b".to_string()]);
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn iter<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    for i in 0..187 {
        vec.push(i.to_string());
    }

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    let iter = unsafe { col.iter(0) };
    assert_eq!(iter.count(), 0);

    let mut iter = unsafe { col.iter(4) };
    for i in 0..4 {
        assert_eq!(iter.next(), Some(&i.to_string()));
    }
    assert_eq!(iter.next(), None);

    let mut iter = unsafe { col.iter(187) };
    for i in 0..187 {
        assert_eq!(iter.next(), Some(&i.to_string()));
    }
    assert_eq!(iter.next(), None);
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn get<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    for i in 0..187 {
        vec.push(i.to_string());
    }

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    assert_eq!(unsafe { col.get(4) }, Some(&String::from("4")));
    assert_eq!(unsafe { col.get(186) }, Some(&String::from("186")));

    let capacity = col.capacity();

    for i in capacity..(capacity + 10) {
        assert_eq!(unsafe { col.get(i) }, None);
    }
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn get_mut<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    for i in 0..187 {
        vec.push(i.to_string());
    }

    let mut col: PinnedConcurrentCol<_, _, MyConState<_>> =
        PinnedConcurrentCol::new_from_pinned(vec);

    let element42 = unsafe { col.get_mut(42) }.expect("is-some");
    *element42 = "x".to_string();
    assert_eq!(unsafe { col.get(42) }, Some(&String::from("x")));
}

#[test_matrix([
    SplitVec::with_doubling_growth(),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth(4),
    SplitVec::with_linear_growth_and_fragments_capacity(4, 33),
    FixedVec::new(51)
])]
fn can_reserve_maximum_capacity<P: IntoConcurrentPinnedVec<String>>(pinned_vec: P) {
    let mut col: PinnedConcurrentCol<_, _, MyConState<_>> =
        PinnedConcurrentCol::new_from_pinned(pinned_vec);

    let max_cap = col.maximum_capacity();
    let requested_max_cap = max_cap + 1;

    let new_max_cap = unsafe { col.reserve_maximum_capacity(0, requested_max_cap) };

    assert!(new_max_cap >= requested_max_cap);
}

#[test_matrix([
    FixedVec::new(222),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
])]
fn clear<P: IntoConcurrentPinnedVec<String>>(mut vec: P) {
    for i in 0..187 {
        vec.push(i.to_string());
    }

    let mut col: PinnedConcurrentCol<_, _, MyConState<_>> =
        PinnedConcurrentCol::new_from_pinned(vec);

    assert_eq!(col.state().initial_len, 187);
    assert_eq!(col.capacity(), col.state().initial_cap);

    unsafe { col.clear(187) };

    assert_eq!(col.state().initial_len, 0);
}
