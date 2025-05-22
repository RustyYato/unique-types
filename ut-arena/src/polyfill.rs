macro_rules! assert_unchecked {
    ($b:expr) => {{
        let b = $b;
        debug_assert!(b);
        core::hint::assert_unchecked(b);
    }};
}
