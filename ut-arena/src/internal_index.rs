///
///
/// # Safety
///
/// to_usize must give the exact usize that was passed to from_usize_unchecked
pub unsafe trait InternalIndex: Copy + core::fmt::Debug {
    /// # Safety
    ///
    /// x must be less or equal to than Self::MAX
    fn from_usize(x: usize) -> Self;

    unsafe fn from_usize_unchecked(x: usize) -> Self;

    /// converts self to a usize
    fn to_usize(self) -> usize;
}

macro_rules! prim {
    ($ty:ident) => {
        unsafe impl InternalIndex for $ty {
            #[inline]
            fn from_usize(x: usize) -> Self {
                x.try_into()
                    .expect("tried to create a Arena with too many elements")
            }

            unsafe fn from_usize_unchecked(x: usize) -> Self {
                debug_assert!(Self::try_from(x).is_ok());
                x as Self
            }

            #[inline]
            fn to_usize(self) -> usize {
                self as usize
            }
        }
    };
}

prim!(u8);
prim!(u16);
prim!(u32);
prim!(u64);
prim!(u128);
prim!(usize);
