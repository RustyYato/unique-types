#![no_std]
#![forbid(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    unsafe_op_in_unsafe_fn,
    missing_docs,
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::alloc_instead_of_core
)]

//! # ut-cell
//!
//! This crate allows accessing interior mutable structures by utilizing unique types

use core::{cell::UnsafeCell, mem};

use unique_types::{TrivialToken, UniqueType};

#[doc(hidden)]
pub use core::result::Result;

/// The error type of try_load_all
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum TryLoadAllError {
    /// The nth argument wasn't owned by the provided owner
    NotOwned {
        /// the argument
        arg: usize,
    },
    /// Two arguments overlapped
    Overlaps {
        /// the first argument
        a: usize,
        /// the second argument
        b: usize,
    },
}

#[doc(hidden)]
pub mod load_all;

impl<T: ?Sized + UniqueType> CellOwner for T {}
/// An extension trait for [`UniqueType`] that allows accessing [`UtCell`]
pub trait CellOwner: UniqueType {
    /// Get a reference to a value in a [`UtCell`]
    ///
    /// # Panics
    ///
    /// * If the cell isn't owned by self
    #[cfg_attr(debug_assertions, track_caller)]
    fn get<'a, T: ?Sized>(&'a self, cell: &'a UtCell<T, Self>) -> &'a T {
        cell.load(self)
    }

    /// Get a mutable reference to a value in a [`UtCell`]
    ///
    /// # Panics
    ///
    /// * If the cell isn't owned by self
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_mut<'a, T: ?Sized>(&'a mut self, cell: &'a UtCell<T, Self>) -> &'a mut T {
        cell.load_mut(self)
    }

    /// Get two mutable reference to a values in [`UtCell`]s
    ///
    /// # Panics
    ///
    /// * If any cell isn't owned by self
    /// * If any cell overlaps with any other cell
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_mut2<'a, T: ?Sized, U: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
    ) -> (&'a mut T, &'a mut U) {
        load_all!( self => a, b )
    }

    /// Get three mutable reference to a values in [`UtCell`]s
    ///
    /// # Panics
    ///
    /// * If any cell isn't owned by self
    /// * If any cell overlaps with any other cell
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_mut3<'a, T: ?Sized, U: ?Sized, V: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
    ) -> (&'a mut T, &'a mut U, &'a mut V) {
        load_all!( self => a, b, c )
    }

    /// Get four mutable reference to a values in [`UtCell`]s
    ///
    /// # Panics
    ///
    /// * If any cell isn't owned by self
    /// * If any cell overlaps with any other cell
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_mut4<'a, T: ?Sized, U: ?Sized, V: ?Sized, X: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
        d: &'a UtCell<X, Self>,
    ) -> (&'a mut T, &'a mut U, &'a mut V, &'a mut X) {
        load_all!( self => a, b, c, d )
    }

    /// Try to get two mutable reference to a values in [`UtCell`]s
    fn try_get_mut2<'a, T: ?Sized, U: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
    ) -> Result<(&'a mut T, &'a mut U), TryLoadAllError> {
        load_all!( self => try a, b )
    }

    /// Try to get three mutable reference to a values in [`UtCell`]s
    fn try_get_mut3<'a, T: ?Sized, U: ?Sized, V: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
    ) -> Result<(&'a mut T, &'a mut U, &'a mut V), TryLoadAllError> {
        load_all!( self => try a, b, c )
    }

    /// Try to get four mutable reference to a values in [`UtCell`]s
    fn try_get_mut4<'a, T: ?Sized, U: ?Sized, V: ?Sized, X: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
        d: &'a UtCell<X, Self>,
    ) -> Result<(&'a mut T, &'a mut U, &'a mut V, &'a mut X), TryLoadAllError> {
        load_all!( self => try a, b, c, d )
    }
}

/// A [`UtCell`] allows accessing references to the interior value
/// when you have a witness unique type that "owns" this [`UtCell`]
#[repr(C)]
pub struct UtCell<T: ?Sized, C: CellOwner + ?Sized> {
    token: C::Token,
    value: UnsafeCell<T>,
}

// SAFETY:
// UtCell doesn't wrap token in an `UnsafeCell` so it can inherit it's Sync requirements
// UtCell expose shared and exclusive reference to T even if you have a shared reference to UtCell
//      so it must require T: Send + Sync
unsafe impl<T: ?Sized, C: CellOwner> Sync for UtCell<T, C>
where
    T: Send + Sync,
    C::Token: Sync,
{
}

fn validate_trivial_token<T: TrivialToken>(get_align: impl FnOnce() -> usize) {
    fn illegal_trivial_token<T>() -> ! {
        panic!(
            "Token {} is not a valid `TrivialToken`",
            core::any::type_name::<T>()
        )
    }

    // ensure that Self is the same size and align as T
    if mem::size_of::<T>() != 0 || mem::align_of::<T>() != 1 && mem::align_of::<T>() > get_align() {
        illegal_trivial_token::<T>()
    }

    // assert that there is a value of C::Token
    let _value: T = TrivialToken::NEW;
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C>
where
    C::Token: TrivialToken,
{
    /// Convert a mutable reference to a value to a mutable reference to a [`UtCell`]
    ///
    /// This can only be done when the Token of the [`CellOwner`] is a
    /// 1 aligned ZST.
    #[inline]
    pub fn from_mut(x: &mut T) -> &mut Self {
        validate_trivial_token::<C::Token>(|| mem::align_of_val(x));

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &mut *(x as *mut T as *mut Self) }
    }
}

impl<T, C: CellOwner + ?Sized> UtCell<[T], C>
where
    C::Token: TrivialToken,
{
    /// Convert a [`UtCell`] of a slice to a slice of [`UtCell`]s
    #[inline]
    pub fn as_slice_of_cells(&self) -> &[UtCell<T, C>] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &*(self as *const Self as *const [UtCell<T, C>]) }
    }

    /// Convert a slice of [`UtCell`]s to a [`UtCell`] of a slice
    #[inline]
    pub fn from_slice_of_cells(slice: &[UtCell<T, C>]) -> &Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &*(slice as *const [UtCell<T, C>] as *const Self) }
    }

    /// Convert a [`UtCell`] of a slice to a slice of [`UtCell`]s
    #[inline]
    pub fn as_slice_of_cells_mut(&mut self) -> &mut [UtCell<T, C>] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &mut *(self as *mut Self as *mut [UtCell<T, C>]) }
    }

    /// Convert a slice of [`UtCell`]s to a [`UtCell`] of a slice
    #[inline]
    pub fn from_slice_of_cells_mut(slice: &mut [UtCell<T, C>]) -> &mut Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &mut *(slice as *mut [UtCell<T, C>] as *mut Self) }
    }
}

impl<T, C: CellOwner + ?Sized, const N: usize> UtCell<[T; N], C>
where
    C::Token: TrivialToken,
{
    /// Convert a [`UtCell`] of an array to an array of [`UtCell`]s
    #[inline]
    pub fn as_array_of_cells(&self) -> &[UtCell<T, C>; N] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &*(self as *const Self as *const [UtCell<T, C>; N]) }
    }

    /// Convert an array of [`UtCell`]s to a [`UtCell`] of an array
    #[inline]
    pub fn from_array_of_cells(array: &[UtCell<T, C>; N]) -> &Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &*(array as *const [UtCell<T, C>; N] as *const Self) }
    }

    /// Convert a [`UtCell`] of an array to an array of [`UtCell`]s
    #[inline]
    pub fn as_array_of_cells_mut(&mut self) -> &mut [UtCell<T, C>; N] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &mut *(self as *mut Self as *mut [UtCell<T, C>; N]) }
    }

    /// Convert an array of [`UtCell`]s to a [`UtCell`] of an array
    #[inline]
    pub fn from_array_of_cells_mut(array: &mut [UtCell<T, C>; N]) -> &mut Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        // SAFETY: validate_trivial_token ensures that the token type is sufficiently
        // aligned, zero sized, and trivial to construct
        unsafe { &mut *(array as *mut [UtCell<T, C>; N] as *mut Self) }
    }
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C> {
    /// Get a mutable reference to the underlying value
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // NOTE: This is safe because all accesses to the underlying value
        // also bind the reference to self. So if we have unique access to self
        // then we also have unique access to T
        self.value.get_mut()
    }

    /// Get a mutable raw pointer to the underlying value
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<T, C: CellOwner + ?Sized> UtCell<T, C> {
    /// Construct a [`UtCell`] from a [`CellOwner`]
    pub fn new(owner: &C, value: T) -> Self {
        Self::from_token(owner.token(), value)
    }

    /// Construct a [`UtCell`] from a token from a [`CellOwner`]
    pub const fn from_token(token: C::Token, value: T) -> Self {
        Self {
            token,
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C> {
    /// Check if this cell is owned by the given [`CellOwner`]
    pub fn is_owned_by(&self, owner: &C) -> bool {
        owner.owns(&self.token)
    }

    /// Check if this cell is owned by the given [`CellOwner`]
    ///
    /// # Panic
    ///
    /// If this type isn't owned by the owner, then this function panics
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn assert_owned_by(&self, owner: &C) {
        #[cfg_attr(debug_assertions, track_caller)]
        fn assert_owned_by_failed<T: ?Sized>() -> ! {
            panic!(
                "Tried to access a {} with a value that doesn't own the cell",
                core::any::type_name::<T>()
            )
        }

        if !self.is_owned_by(owner) {
            assert_owned_by_failed::<Self>()
        }
    }

    /// Load a reference from this cell
    ///
    /// # Panic
    ///
    /// If this type isn't owned by the owner, then this function panics
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn load<'a>(&'a self, owner: &'a C) -> &'a T {
        self.assert_owned_by(owner);
        // SAFETY:
        // [`UniqueToken`] ensures that all references to an owner that owns this type
        // must point to the exact same value
        // since we have a shared reference to the owner and
        // to self. The output reference can't outlive either of those references
        // This ensures that if someone tries to call [`get_mut`], or [`load_mut`],
        // then they must give up the output reference. Since those functions require
        // an exclusive reference to either self or the owner
        unsafe { &*self.as_ptr() }
    }

    /// Load an mutable reference from this cell
    ///
    /// # Panic
    ///
    /// If this type isn't owned by the owner, then this function panics
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn load_mut<'a>(&'a self, owner: &'a mut C) -> &'a mut T {
        self.assert_owned_by(owner);
        // SAFETY:
        // [`UniqueToken`] ensures that all references to an owner that owns this type
        // must point to the exact same value
        // since we have a shared reference to the owner and
        // to self. The output reference can't outlive either of those references
        // This ensures that if someone tries to call [`get_mut`], [`load_mut`], or [`load`]
        // then they must give up the output reference. Since those functions require
        // an exclusive reference to either self or the owner
        unsafe { &mut *self.as_ptr() }
    }

    #[doc(hidden)]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn load_mut_unchecked<'a>(&'a self, _owner: &'a C) -> &'a mut T {
        // SAFETY: the caller ensures that _owner owns this value
        // and that the output exclusive reference won't be invalidated for it's
        // entire lifetime
        unsafe { &mut *self.as_ptr() }
    }
}
