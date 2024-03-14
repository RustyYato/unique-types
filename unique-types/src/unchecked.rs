//! Unchecked unique types rely on the user to unsure that they are used correctly
//!
//! This mostly exists to plug into existing interfaces where it is onerous to formally
//! prove to the type system that the type is unique, but you can ensure that it is.

/// A unique type which relies on the created of the type to ensure that it is in fact unique
///
/// see module docs for rationale
pub struct UncheckedUniqueType<const IS_UNIQUE_TOKEN: bool = false>(());

/// The token for an [`UncheckedUniqueType`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UncheckedToken;

impl core::fmt::Debug for UncheckedUniqueType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("UncheckedUniqueType")
    }
}

impl UncheckedUniqueType {
    /// Create a new [`UncheckedUniqueType`]
    ///
    /// # Safety
    ///
    /// You must uphold the guarantees in [`UniqueType`](crate::UniqueType)
    #[inline]
    pub const unsafe fn new() -> Self {
        Self(())
    }
}

impl UncheckedUniqueType<true> {
    /// Create a new [`UncheckedUniqueType`]
    ///
    /// # Safety
    ///
    /// You must uphold the guarantees in [`UniqueType`](crate::UniqueType) and [`UniqueToken`](crate::UniqueToken)
    #[inline]
    pub const unsafe fn new_unique_token() -> Self {
        Self(())
    }
}

// SAFETY: upheld because creator of this value ensured it in
// [`UncheckedUniqueType::new`] or [`UncheckedUniqueType::new_unique_token`]
unsafe impl<const IS_UNIQUE_TOKEN: bool> crate::UniqueType
    for UncheckedUniqueType<IS_UNIQUE_TOKEN>
{
    type Token = UncheckedToken;

    #[inline]
    fn token(&self) -> Self::Token {
        UncheckedToken
    }

    #[inline]
    fn owns(&self, _token: &Self::Token) -> bool {
        true
    }
}

// SAFETY: upheld because creator of this value ensured it in
// [`UncheckedUniqueType::new_unique_token`]
unsafe impl crate::UniqueToken for UncheckedUniqueType<true> {}
