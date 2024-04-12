mod state;

use orx_pinned_concurrent_col::*;
use orx_pinned_vec::PinnedVec;
use orx_split_vec::SplitVec;
use state::MyConState;

#[test]
fn write_n_items() {
    let mut vec: SplitVec<String> = SplitVec::with_doubling_growth_and_fragments_capacity(32);
    vec.push("a".to_string());
    vec.push("b".to_string());

    let col: PinnedConcurrentCol<_, _, MyConState> = PinnedConcurrentCol::new_from_pinned(vec);

    for idx in (2..1602).step_by(4) {
        unsafe { col.write_n_items(idx, 4, (idx..(idx + 4)).map(|x| x.to_string())) };
    }

    assert_eq!(unsafe { col.get(0) }, Some(&String::from("a")));
    assert_eq!(unsafe { col.get(1) }, Some(&String::from("b")));
    for idx in 2..1602 {
        assert_eq!(unsafe { col.get(idx) }, Some(&idx.to_string()));
    }

    let vec = unsafe { col.into_inner(1602) };
    assert_eq!(vec.get(0), Some(&String::from("a")));
    assert_eq!(vec.get(1), Some(&String::from("b")));
    for idx in 2..1602 {
        assert_eq!(vec.get(idx), Some(&idx.to_string()));
    }
}
