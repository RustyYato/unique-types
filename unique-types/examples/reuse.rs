use unique_types::{reusable_runtime::ReuseRuntimeUt, UniqueType};

unique_types::custom_counter! {
    struct MyType(core::num::NonZeroU8);
}

fn main() {
    let b = ReuseRuntimeUt::<MyType>::with_counter();
    let a = ReuseRuntimeUt::<MyType>::with_counter();

    let a_token = a.token();
    assert!(!a.owns(&b.token()));
    assert!(!b.owns(&a.token()));

    drop(a);

    // since a was created last and is dropped, we will reuse it's token
    let c = ReuseRuntimeUt::<MyType>::with_counter();
    assert!(c.owns(&a_token));
}
