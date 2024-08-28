mod state;
use orx_fixed_vec::FixedVec;
use orx_pinned_concurrent_col::*;
use orx_pinned_vec::{IntoConcurrentPinnedVec, PinnedVec};
use orx_split_vec::SplitVec;
use state::MyConState;

#[test]
fn with_doubling_growth() {
    let col: PinnedConcurrentCol<String, _, MyConState<_>> =
        PinnedConcurrentCol::with_doubling_growth();

    assert_eq!(col.capacity(), 4);
    assert_eq!(col.maximum_capacity(), 17_179_869_180);
    assert_eq!(col.state().initial_len, 0);
    assert_eq!(col.state().initial_cap, 4);
}

#[test]
fn with_linear_growth() {
    let col: PinnedConcurrentCol<String, _, MyConState<_>> =
        PinnedConcurrentCol::with_linear_growth(4, 10);

    assert_eq!(col.capacity(), 2usize.pow(4));
    assert_eq!(col.maximum_capacity(), 2usize.pow(4) * 10);
    assert_eq!(col.state().initial_len, 0);
    assert_eq!(col.state().initial_cap, 2usize.pow(4));
}

#[test]
fn with_fixed_capacity() {
    let col: PinnedConcurrentCol<String, _, MyConState<_>> =
        PinnedConcurrentCol::with_fixed_capacity(5648);

    assert_eq!(col.capacity(), 5648);
    assert_eq!(col.maximum_capacity(), 5648);
    assert_eq!(col.state().initial_len, 0);
    assert_eq!(col.state().initial_cap, 5648);
}

#[test]
fn from() {
    fn validate<P: IntoConcurrentPinnedVec<String>>(pinned_vec: P) {
        let max_cap = pinned_vec.capacity_state().maximum_concurrent_capacity();
        let expected_con_state = MyConState::<_>::new_for_pinned_vec(&pinned_vec);
        let col: PinnedConcurrentCol<_, _, MyConState<_>> =
            PinnedConcurrentCol::new_from_pinned(pinned_vec);

        assert_eq!(col.capacity(), expected_con_state.initial_cap);
        assert_eq!(col.maximum_capacity(), max_cap);
        assert_eq!(col.state().initial_len, expected_con_state.initial_len);
        assert_eq!(col.state().initial_cap, expected_con_state.initial_cap);
    }

    validate(SplitVec::new());
    validate(SplitVec::with_doubling_growth());
    validate(SplitVec::with_doubling_growth_and_fragments_capacity(32));
    validate(SplitVec::with_linear_growth(10));
    validate(SplitVec::with_linear_growth_and_fragments_capacity(10, 10));
    validate(FixedVec::new(1024));

    let mut vec = SplitVec::with_doubling_growth_and_fragments_capacity(32);
    for _ in 0..1234 {
        vec.push("hello".to_string());
    }
    validate(vec);
}
