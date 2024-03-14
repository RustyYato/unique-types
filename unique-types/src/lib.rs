#![no_std]
#![forbid(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    unsafe_op_in_unsafe_fn,
    missing_docs
)]

//! # unique-types
//!
//! A crate to create and manage types who's values are all unique

#[macro_use]
mod macros;

pub mod lifetime;
pub mod runtime;
pub mod unchecked;
pub mod unique_indices;

/// A type where all values of the type are distinct from each other.
///
/// # Safety
///
/// It should be impossible to trigger any of the asserts in the following function
///
/// ```
/// # use unique_types::UniqueType;
/// fn no_duplicates<T: UniqueType>(a: T, b: T) {
///     assert!(a.token() != b.token());
///     assert!( !a.owns(&b.token()) );
///     assert!( !b.owns(&a.token()) );
/// }
/// ```
pub unsafe trait UniqueType {
    /// A token type which is cheap to share around
    type Token: Copy + PartialEq;

    /// Get the token for this type
    fn token(&self) -> Self::Token;

    /// Check a given token is owns by this value
    ///
    /// NOTE: this may not be the value which created the token so long the value which created the
    /// token is inaccessible beforehand.
    fn owns(&self, token: &Self::Token) -> bool;
}

/// A marker trait that guarantees that [`UniqueType::owns`] only returns true for value that
/// created the token.
///
/// For example, if you use a `T: `[`UniqueType`] for unchecked indexing, then you should also require
/// `T: `[`UniqueToken`] to ensure that after the data structure is destroyed and re-created, it is
/// impossible to use old indices in the new data structure.
///
/// # Safety
///
/// `Self::Token` is only ever [owned](UniqueType::owns) by the value that created the token
///
/// For example, [`&mut T`](https://doc.rust-lang.org/std/primitive.reference.html) cannot implement [`UniqueToken`]
/// [`UniqueType::owns`] would return [`true`] for tokens that `T` created. This violates the
/// condition above that only the value that created the value owns the token.
pub unsafe trait UniqueToken: UniqueType {}

/// SAFETY: &mut T gets unique access to the value of `T`, and every value of `T` is distinct
/// so by transitivity, all values of `&mut T` are distinct
unsafe impl<T: UniqueType + ?Sized> UniqueType for &mut T {
    type Token = T::Token;

    #[inline]
    fn token(&self) -> Self::Token {
        T::token(self)
    }

    #[inline]
    fn owns(&self, token: &Self::Token) -> bool {
        T::owns(self, token)
    }
}
