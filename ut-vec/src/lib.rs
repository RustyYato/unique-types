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

//! # ut-vec
//!
//! [`UtVec`] is a append-only vector when used with a [`UniqueToken`]

extern crate alloc;

use core::{
    ops::{self, RangeBounds},
    ptr::NonNull,
};

use alloc::{collections::TryReserveError, vec::Vec};

use unique_types::UniqueToken;

/// An append only vector
pub struct UtVec<T, O: ?Sized = ()> {
    data: Vec<T>,
    owner: O,
}

/// An index into the [`UtVec`] that owns this index
pub struct UtIndex<O: ?Sized + UniqueToken> {
    token: O::Token,
    index: usize,
}

impl<O: ?Sized + UniqueToken> UtIndex<O> {
    /// Get the underlying index
    pub const fn get(&self) -> usize {
        self.index
    }
}

impl<T, O> UtVec<T, O> {
    /// Create a [`UtVec`] from raw parts
    #[inline]
    pub const fn new(owner: O) -> Self {
        Self::from_parts(Vec::new(), owner)
    }

    /// Create a [`UtVec`] from raw parts
    #[inline]
    pub const fn from_parts(data: Vec<T>, owner: O) -> Self {
        Self { data, owner }
    }

    /// Extract the vector from the [`UtVec`]
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// Extract the vector and owner from the [`UtVec`]
    ///
    /// # Safety
    ///
    /// If you construct a [`UtVec`] from the owner again you must ensure that
    /// all [`UtIndex`]s created for self that are used for the new [`UtVec`]
    /// are in bounds.
    ///
    /// An easy way to ensure this is to not use the old indices at all or to recreate the
    /// [`UtVec`] with the same vector without removing any elements of the vector
    #[inline]
    pub unsafe fn into_parts(self) -> (Vec<T>, O) {
        (self.data, self.owner)
    }
}

impl<T, O: ?Sized> UtVec<T, O> {
    /// Get a mutable reference to the underlying vector
    ///
    /// # Safety
    ///
    /// You must not reduce the size of the vector
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<T> {
        &mut self.data
    }

    /// see [`Vec::as_slice`]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// see [`Vec::as_mut_slice`]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// see [`Vec::len`]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// see [`Vec::is_empty`]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// see [`Vec::capacity`]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    /// see [`Vec::reserve`]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional)
    }

    /// see [`Vec::reserve_exact`]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional)
    }

    /// see [`Vec::try_reserve`]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.data.try_reserve(additional)
    }

    /// see [`Vec::try_reserve_exact`]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.data.try_reserve_exact(additional)
    }

    /// see [`Vec::push`]
    pub fn push(&mut self, value: T) {
        self.data.push(value)
    }

    /// see [`Vec::append`]
    pub fn append(&mut self, vec: &mut Vec<T>) {
        self.data.append(vec)
    }

    /// Add `additional` new elements of `value` to the vector
    ///
    /// see [`Vec::resize`]
    pub fn grow(&mut self, additional: usize, value: T)
    where
        T: Clone,
    {
        self.reserve(additional);
        self.data.resize(self.len() + additional, value);
    }

    /// Add `additional` new elements by calling `make_value` to the vector
    ///
    /// see [`Vec::resize_with`]
    pub fn grow_with(&mut self, additional: usize, make_value: impl FnMut() -> T) {
        self.reserve(additional);
        self.data.resize_with(self.len() + additional, make_value);
    }

    /// see [`Vec::extend_from_slice`]
    pub fn extend_from_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        self.data.extend_from_slice(slice)
    }

    /// see [`Vec::extend_from_slice`]
    pub fn extend_from_within<R>(&mut self, range: R)
    where
        R: RangeBounds<usize>,
        T: Clone,
    {
        self.data.extend_from_within(range)
    }
}

impl<T, A, O> Extend<A> for UtVec<T, O>
where
    Vec<T>: Extend<A>,
{
    fn extend<I: IntoIterator<Item = A>>(&mut self, iter: I) {
        self.data.extend(iter)
    }
}

impl<T, O: ?Sized> ops::Deref for UtVec<T, O> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, O: ?Sized> ops::DerefMut for UtVec<T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, O, I: UtVecIndex<O>> ops::Index<I> for UtVec<T, O> {
    type Output = I::Output<T>;

    fn index(&self, index: I) -> &Self::Output {
        match index.is_in_bounds(self.len(), &self.owner) {
            Err(err) => err.handle(),
            // SAFETY: UtVecIndex guarantees that offset_slice will return a valid pointer
            // into a subset of the slice
            Ok(()) => unsafe {
                let slice = NonNull::from(self.data.as_slice());
                let slice = index.offset_slice(slice, &self.owner);
                &*slice.as_ptr()
            },
        }
    }
}

impl<T, O, I: UtVecIndex<O>> ops::IndexMut<I> for UtVec<T, O> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        match index.is_in_bounds(self.len(), &self.owner) {
            Err(err) => err.handle(),
            // SAFETY: UtVecIndex guarantees that offset_slice will return a valid pointer
            // into a subset of the slice
            Ok(()) => unsafe {
                let slice = NonNull::from(self.data.as_mut_slice());
                let slice = index.offset_slice(slice, &self.owner);
                &mut *slice.as_ptr()
            },
        }
    }
}

use seal::Seal;
mod seal {
    pub trait Seal {}
}

/// The error type of [`UtVecIndex::is_in_bounds`]
pub enum IndexError {
    /// If the index is not owned by the [`UtVec`]
    NotOwned,
    /// If the index is not in bounds of the [`UtVec`]
    NotInBounds {
        /// The index that was accessed
        index: usize,
        /// The length of the [`UtVec`]
        len: usize,
        /// If the index is inclusive
        is_inclusive: bool,
    },
    /// If the range bounds are out of order
    OutOfOrder {
        /// The start of the range
        start: usize,
        /// The end of the range
        end: usize,
    },
}

impl IndexError {
    #[cold]
    #[inline(never)]
    fn handle(self) -> ! {
        match self {
            IndexError::NotOwned => panic!("Index not owned by `UtVec`"),
            IndexError::NotInBounds {
                index,
                len,
                is_inclusive: false,
            } => panic!("Index out of bounds (index > length), index: {index}, length: {len}"),
            IndexError::NotInBounds {
                index,
                len,
                is_inclusive: true,
            } => panic!("Index out of bounds (index >= length), index: {index}, length: {len}"),
            IndexError::OutOfOrder { start, end } => {
                panic!("Range bounds out of order (start > end), start: {start}, end: {end}")
            }
        }
    }
}

/// Any type which can be used to index into a [`UtVec`]
///
/// This includes, usize, ranges over usize, [`UtIndex`], and ranges over [`UtIndex`]
pub trait UtVecIndex<O: ?Sized>: Seal {
    /// The output type,
    /// will be T if the type is not a range
    /// will be \[T\] if the type is a range
    type Output<T>: ?Sized;

    /// Check if this index is in bounds of the [`UtVec`] that owns `O`
    fn is_in_bounds(&self, len: usize, owner: &O) -> Result<(), IndexError>;

    /// Indexes into the slice without checking if self is in bounds
    ///
    /// # Safety
    ///
    /// `slice` must in a single allocated for it's entire length
    /// `is_in_bounds` must return Ok when passed the length of the slice and
    /// the owner associated with the slice
    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>>;
}

impl Seal for ops::RangeFull {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeFull {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, _owner: &O) -> Result<(), IndexError> {
        Ok(())
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, _owner: &O) -> NonNull<Self::Output<T>> {
        slice
    }
}

impl Seal for usize {}
impl<O: ?Sized> UtVecIndex<O> for usize {
    type Output<T> = T;

    fn is_in_bounds(&self, len: usize, _owner: &O) -> Result<(), IndexError> {
        if *self < len {
            Ok(())
        } else {
            Err(IndexError::NotInBounds {
                index: *self,
                len,
                is_inclusive: true,
            })
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, _owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: is_in_bounds checks that self is in bounds of the slice length
        // and the slice is in a single allocation for it's entire, so offseting
        // somewhere inside that length if fine
        unsafe { NonNull::new_unchecked(slice.as_ptr().cast::<T>().add(self)) }
    }
}

impl Seal for ops::RangeTo<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeTo<usize> {
    type Output<T> = [T];

    fn is_in_bounds(&self, len: usize, _owner: &O) -> Result<(), IndexError> {
        if self.end <= len {
            Ok(())
        } else {
            Err(IndexError::NotInBounds {
                index: self.end,
                len,
                is_inclusive: false,
            })
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, _owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                slice.as_ptr().cast(),
                self.end,
            ))
        }
    }
}

impl Seal for ops::RangeToInclusive<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeToInclusive<usize> {
    type Output<T> = [T];

    fn is_in_bounds(&self, len: usize, _owner: &O) -> Result<(), IndexError> {
        if self.end < len {
            Ok(())
        } else {
            Err(IndexError::NotInBounds {
                index: self.end,
                len,
                is_inclusive: true,
            })
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, _owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                slice.as_ptr().cast(),
                self.end + 1,
            ))
        }
    }
}

impl Seal for ops::RangeFrom<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeFrom<usize> {
    type Output<T> = [T];

    fn is_in_bounds(&self, len: usize, _owner: &O) -> Result<(), IndexError> {
        if self.start <= len {
            Ok(())
        } else {
            Err(IndexError::NotInBounds {
                index: self.start,
                len,
                is_inclusive: false,
            })
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, _owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                slice.as_ptr().cast::<T>().add(self.start),
                slice.len() - self.start,
            ))
        }
    }
}

impl Seal for ops::Range<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::Range<usize> {
    type Output<T> = [T];

    fn is_in_bounds(&self, len: usize, owner: &O) -> Result<(), IndexError> {
        if self.start > self.end {
            Err(IndexError::OutOfOrder {
                start: self.start,
                end: self.end,
            })
        } else {
            // we don't need to check that start is in bounds since it is <= end
            // (self.start..).is_in_bounds(len, owner)?;
            (..self.end).is_in_bounds(len, owner)?;
            Ok(())
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            let slice = (..self.end).offset_slice(slice, owner);
            (self.start..).offset_slice(slice, owner)
        }
    }
}

impl Seal for ops::RangeInclusive<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeInclusive<usize> {
    type Output<T> = [T];

    fn is_in_bounds(&self, len: usize, owner: &O) -> Result<(), IndexError> {
        if self.start() > self.end() {
            Err(IndexError::OutOfOrder {
                start: *self.start(),
                end: *self.end(),
            })
        } else {
            // we don't need to check that start is in bounds since it is <= end
            // (self.start..).is_in_bounds(len, owner)?;
            (..=*self.end()).is_in_bounds(len, owner)?;
            Ok(())
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            let slice = (..=*self.end()).offset_slice(slice, owner);
            (*self.start()..).offset_slice(slice, owner)
        }
    }
}

impl<O: ?Sized + UniqueToken> Seal for UtIndex<O> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for UtIndex<O> {
    type Output<T> = T;

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { self.index.offset_slice(slice, owner) }
    }
}

impl<O: ?Sized + UniqueToken> Seal for ops::RangeTo<UtIndex<O>> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeTo<UtIndex<O>> {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.end.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (..self.end.index).offset_slice(slice, owner) }
    }
}

impl<O: ?Sized + UniqueToken> Seal for ops::RangeToInclusive<UtIndex<O>> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeToInclusive<UtIndex<O>> {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.end.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (..=self.end.index).offset_slice(slice, owner) }
    }
}

impl<O: ?Sized + UniqueToken> Seal for ops::RangeFrom<UtIndex<O>> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeFrom<UtIndex<O>> {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.start.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start.index..).offset_slice(slice, owner) }
    }
}

impl<O: ?Sized + UniqueToken> Seal for ops::Range<UtIndex<O>> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::Range<UtIndex<O>> {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if self.start.index > self.end.index {
            Err(IndexError::OutOfOrder {
                start: self.start.index,
                end: self.end.index,
            })
        } else if owner.owns(&self.start.token) && owner.owns(&self.end.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start.index..self.end.index).offset_slice(slice, owner) }
    }
}

impl<O: ?Sized + UniqueToken> Seal for ops::RangeInclusive<UtIndex<O>> {}
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeInclusive<UtIndex<O>> {
    type Output<T> = [T];

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if self.start().index > self.end().index {
            Err(IndexError::OutOfOrder {
                start: self.start().index,
                end: self.end().index,
            })
        } else if owner.owns(&self.start().token) && owner.owns(&self.end().token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(self, slice: NonNull<[T]>, owner: &O) -> NonNull<Self::Output<T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start().index..=self.end().index).offset_slice(slice, owner) }
    }
}
