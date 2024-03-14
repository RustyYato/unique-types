use unique_types::runtime::RuntimeUt;

unique_types::custom_counter! {
    struct MyType;
}

fn main() {
    let rt = RuntimeUt::<MyType>::with_counter();
    assert!(RuntimeUt::<MyType>::try_with_counter().is_none());
    assert_eq!(core::mem::size_of_val(&rt), 0);
    assert_eq!(core::mem::align_of_val(&rt), 1);
}
