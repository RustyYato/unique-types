use unique_types::{reusable_runtime::ReuseRuntimeUt, UniqueType};

unique_types::custom_counter! {
    struct MyType(core::num::NonZeroU8);
    with_reuse std::sync::Mutex<Vec<core::num::NonZeroU8>>
}

fn main() {
    let b = ReuseRuntimeUt::<MyType>::with_counter();
    let a = ReuseRuntimeUt::<MyType>::with_counter();

    let a_token = a.token();
    let b_token = b.token();
    assert!(!a.owns(&b.token()));
    assert!(!b.owns(&a.token()));

    drop(a);
    drop(b);

    // since a was created last and is dropped, we will reuse it's token
    let c = ReuseRuntimeUt::<MyType>::with_counter();
    assert!(c.owns(&b_token));

    // since a was created last and is dropped, we will reuse it's token
    let d = ReuseRuntimeUt::<MyType>::with_counter();
    assert!(d.owns(&a_token));
}
