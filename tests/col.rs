mod state;

use orx_fixed_vec::FixedVec;
use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use state::MyConState;
use test_case::test_matrix;

#[test]
fn new_from_pinned() {
    let pinned_vec: SplitVec<String> = SplitVec::with_doubling_growth_and_fragments_capacity(32);
    let expected_state = MyConState::new_for_pinned_vec(&pinned_vec);

    let col: PinnedConcurrentCol<_, _, MyConState> =
        PinnedConcurrentCol::new_from_pinned(pinned_vec.clone());

    assert_eq!(col.state(), &expected_state);
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

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    let vec_back = unsafe { col.into_inner(2) };
    assert_eq!(&vec_back, &["a".to_string(), "b".to_string()]);
}

#[test]
fn zeroes_memory_on_allocation() {
    let vec: SplitVec<String> = SplitVec::new();
    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);
    assert_eq!(col.zeroes_memory_on_allocation(), MyConState::zero_memory());
}

#[test]
fn iter() {
    let mut vec: SplitVec<usize> = SplitVec::new();
    for i in 0..187 {
        vec.push(i);
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    let mut iter = unsafe { col.iter(4) };
    for i in 0..4 {
        assert_eq!(iter.next(), Some(&i));
    }
    assert_eq!(iter.next(), None);

    let mut iter = unsafe { col.iter(187) };
    for i in 0..187 {
        assert_eq!(iter.next(), Some(&i));
    }
    assert_eq!(iter.next(), None);
}

#[test]
fn get() {
    let mut vec: SplitVec<String> = SplitVec::new();
    for i in 0..187 {
        vec.push(i.to_string());
    }

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    assert_eq!(unsafe { col.get(4) }, Some(&String::from("4")));
    assert_eq!(unsafe { col.get(186) }, Some(&String::from("186")));

    let capacity = col.capacity();

    for i in capacity..(capacity + 10) {
        assert_eq!(unsafe { col.get(i) }, None);
    }
}

#[test_matrix([
    SplitVec::with_doubling_growth(),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_recursive_growth(),
    SplitVec::with_recursive_growth_and_fragments_capacity(31),
    SplitVec::with_linear_growth(4),
    SplitVec::with_linear_growth_and_fragments_capacity(4, 33)
])]
fn can_reserve_maximum_capacity<P: PinnedVec<String>>(pinned_vec: P) {
    let mut col: PinnedConcurrentCol<_, _, MyConState> = pinned_vec.into();

    let max_cap = col.maximum_capacity();
    let requested_max_cap = max_cap + 1;

    let new_max_cap = col
        .reserve_maximum_capacity(requested_max_cap)
        .expect("must-be-ok");

    assert!(new_max_cap >= requested_max_cap);
}

#[test_matrix([
    FixedVec::new(51)
])]
fn fails_to_reserve_maximum_capacity<P: PinnedVec<String>>(pinned_vec: P) {
    let mut col: PinnedConcurrentCol<_, _, MyConState> = pinned_vec.into();

    let max_cap = col.maximum_capacity();
    let requested_max_cap = max_cap + 1;

    let result = col.reserve_maximum_capacity(requested_max_cap);

    assert!(result.is_err());
}

#[test_matrix([
    SplitVec::with_doubling_growth_and_fragments_capacity(17),
    SplitVec::with_doubling_growth_and_fragments_capacity(18),
    SplitVec::with_doubling_growth_and_fragments_capacity(19),
])]
#[should_panic]
fn panics_on_reserve_maximum_capacity<P: PinnedVec<String>>(pinned_vec: P) {
    let mut col: PinnedConcurrentCol<_, _, MyConState> = pinned_vec.into();

    let max_cap = col.maximum_capacity();
    let requested_max_cap = max_cap + 1;

    let new_max_cap = col
        .reserve_maximum_capacity(requested_max_cap)
        .expect("must-be-ok");

    assert!(new_max_cap >= requested_max_cap);
}

#[test]
fn clear() {
    let mut vec: SplitVec<usize> = SplitVec::new();
    for i in 0..187 {
        vec.push(i);
    }

    let mut col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    assert_eq!(col.state().initial_len, 187);
    assert_eq!(col.capacity(), col.state().initial_cap);

    col.clear();

    assert_eq!(col.state().initial_len, 0);
    assert_eq!(col.state().initial_cap, 4);
    assert_eq!(col.capacity(), 4);
}
