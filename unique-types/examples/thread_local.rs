#[cfg(feature = "std")]
unique_types::thread_local_custom_counter! {
    pub struct TlGlobal(core::num::NonZeroU8);
}

#[cfg(feature = "std")]
fn main() {
    use unique_types::UniqueType;

    let a = unique_types::runtime::RuntimeUt::<TlGlobal>::with_counter();
    let b = unique_types::runtime::RuntimeUt::<TlGlobal>::with_counter();

    assert!(a.token() != b.token());
    assert!(!a.owns(&b.token()));
    assert!(!b.owns(&a.token()));

    let a_token = a.token();

    drop(a);

    let c = unique_types::runtime::RuntimeUt::<TlGlobal>::with_counter();

    assert!(c.token() != a_token);
    assert!(!c.owns(&a_token));
}

#[cfg(not(feature = "std"))]
fn main() {
    panic!("run this example with --feature std enabled")
}
