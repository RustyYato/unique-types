//! see [`InternalIndex`]

/// A basic integer type
///
/// This is used to reduce the memory usage of arenas which store small elements
///
/// # Safety
///
/// to_usize must give the exact usize that was passed to from_usize_unchecked
pub unsafe trait InternalIndex: Copy + core::fmt::Debug + crate::seal::Seal {
    /// Tries to convert a usize to Self, panicking if it is too large
    ///
    /// # Panics
    ///
    /// x must be less or equal to than Self::MAX
    fn from_usize(x: usize) -> Self;

    /// Casts from usize to Self without checking if usize is too large
    ///
    /// # Safety
    ///
    /// if x was passed to `from_usize`, it must ont panic
    unsafe fn from_usize_unchecked(x: usize) -> Self;

    /// converts self to a usize, and will preserve any legal values passed to [`InternalIndex::from_usize`] or
    /// [`InternalIndex::from_usize_unchecked`]
    fn to_usize(self) -> usize;
}

macro_rules! prim {
    ($ty:ident) => {
        impl crate::seal::Seal for $ty {}
        // SAFETY: TryInto ensures that the usize is in bounds of Self
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
