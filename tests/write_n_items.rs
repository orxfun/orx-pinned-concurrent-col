mod state;

use orx_fixed_vec::FixedVec;
use orx_pinned_concurrent_col::*;
use orx_split_vec::SplitVec;
use prelude::IntoConcurrentPinnedVec;
use state::MyConState;
use test_case::test_matrix;

#[test_matrix([
    FixedVec::new(2132),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
], [1, 20, 151])]
fn write_n_item_drop_col<P: IntoConcurrentPinnedVec<String>>(mut vec: P, n: usize) {
    let len1 = 5;
    let len2 = 1574;

    vec.clear();
    for idx in 0..len1 {
        vec.push(idx.to_string());
    }

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    for idx in (len1..len2).step_by(n) {
        let begin_idx = idx;
        let end_idx = (begin_idx + n).min(len2);
        let len = end_idx - begin_idx;
        let extend = (begin_idx..end_idx).map(|x| x.to_string());

        unsafe { col.write_n_items(begin_idx, len, extend) };
    }

    for idx in 0..len2 {
        assert_eq!(unsafe { col.get(idx) }, Some(&idx.to_string()));
    }

    col.state().set_final_len(len2);
}

#[test_matrix([
    FixedVec::new(2132),
    SplitVec::with_doubling_growth_and_fragments_capacity(16),
    SplitVec::with_linear_growth_and_fragments_capacity(10, 33)
], [1, 20, 151])]
fn write_n_item_drop_vec<P: IntoConcurrentPinnedVec<String>>(mut vec: P, n: usize) {
    let len1 = 5;
    let len2 = 1574;

    vec.clear();
    for idx in 0..len1 {
        vec.push(idx.to_string());
    }

    let col: PinnedConcurrentCol<_, _, MyConState<_>> = PinnedConcurrentCol::new_from_pinned(vec);

    for idx in (len1..len2).step_by(n) {
        let begin_idx = idx;
        let end_idx = (begin_idx + n).min(len2);
        let len = end_idx - begin_idx;
        let extend = (begin_idx..end_idx).map(|x| x.to_string());

        unsafe { col.write_n_items(begin_idx, len, extend) };
    }
    for idx in 0..len2 {
        assert_eq!(unsafe { col.get(idx) }, Some(&idx.to_string()));
    }

    col.state().set_final_len(len2);
    let vec = unsafe { col.into_inner(len2) };

    for idx in 0..len2 {
        assert_eq!(vec.get(idx), Some(&idx.to_string()));
    }
}
