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

#[cfg(feature = "unique-types")]
use unique_types::UniqueToken;

/// An append only vector
#[derive(Debug)]
pub struct UtVec<T, O: ?Sized = ()> {
    data: Vec<T>,
    owner: O,
}

#[cfg(feature = "unique-types")]
/// An index into the [`UtVec`] that owns this index
pub struct UtIndex<O: ?Sized + UniqueToken> {
    token: O::Token,
    index: usize,
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Copy for UtIndex<O> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Clone for UtIndex<O> {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtIndex<O> {
    /// Get the underlying index
    #[inline]
    pub const fn get(&self) -> usize {
        self.index
    }

    /// # Safety
    ///
    /// The index must be in bounds of the [`UtVec`] that is owns the owner
    #[inline]
    pub unsafe fn new_unchecked(index: usize, owner: &O) -> Self {
        Self {
            index,
            token: owner.token(),
        }
    }
}

impl<T> UtVec<T> {
    /// Create an empty [`UtVec`]
    #[inline]
    pub const fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    /// Create a [`UtVec`] from a [`Vec`]
    pub const fn from_vec(data: Vec<T>) -> Self {
        Self { data, owner: () }
    }
}

impl<T> Default for UtVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "unique-types")]
impl<T, O: UniqueToken> UtVec<T, O> {
    /// Create an empty [`UtVec`] with the given owner
    #[inline]
    #[cfg(feature = "unique-types")]
    pub const fn from_owner(owner: O) -> Self {
        Self::from_parts(Vec::new(), owner)
    }

    /// Create a [`UtVec`] from raw parts
    #[inline]
    #[cfg(feature = "unique-types")]
    pub const fn from_parts(data: Vec<T>, owner: O) -> Self {
        Self { data, owner }
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

impl<T, O> UtVec<T, O> {
    /// Extract the vector from the [`UtVec`]
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.data
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
    pub fn owner(&self) -> &O {
        &self.owner
    }

    /// see [`Vec::as_slice`]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// see [`Vec::as_mut_slice`]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// see [`Vec::as_mut_slice`]
    pub fn as_mut_slice_and_owner(&mut self) -> (&mut [T], &O) {
        (&mut self.data, &self.owner)
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

    /// see [`slice::get_unchecked`]
    ///
    /// # Safety
    ///
    /// The index must be in bounds and if it's a range, the start <= end
    pub unsafe fn get_unchecked<I: UtVecIndex<O>>(&self, index: I) -> &GetOutputType<I, O, T> {
        debug_assert!(index.is_in_bounds(self.len(), self.owner()).is_ok());

        let slice = NonNull::from(self.data.as_slice());
        // SAFETY: the caller ensures that this is safe
        let slice = unsafe { index.offset_slice(slice, &self.owner) };
        // SAFETY: UtVecIndex guarantees that offset_slice will return a valid pointer
        // into a subset of the slice
        unsafe { &*slice.as_ptr() }
    }

    /// see [`slice::get_unchecked_mut`]
    ///
    /// # Safety
    ///
    /// The index must be in bounds and if it's a range, the start <= end
    pub unsafe fn get_unchecked_mut<I: UtVecIndex<O>>(
        &mut self,
        index: I,
    ) -> &mut GetOutputType<I, O, T> {
        debug_assert!(index.is_in_bounds(self.len(), self.owner()).is_ok());

        let slice = NonNull::from(self.data.as_mut_slice());
        // SAFETY: the caller ensures that this is safe
        let slice = unsafe { index.offset_slice(slice, &self.owner) };
        // SAFETY: UtVecIndex guarantees that offset_slice will return a valid pointer
        // into a subset of the slice
        unsafe { &mut *slice.as_ptr() }
    }

    /// see [`Vec::extend_from_slice`]
    pub fn get<I: UtVecIndex<O>>(&self, index: I) -> Option<&GetOutputType<I, O, T>> {
        if index.is_in_bounds(self.len(), &self.owner).is_ok() {
            // SAFETY: index.is_in_bounds checks that the index is in bounds, and ranges are well
            // ordered
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    /// see [`Vec::extend_from_slice`]
    pub fn get_mut<I: UtVecIndex<O>>(&mut self, index: I) -> Option<&mut GetOutputType<I, O, T>> {
        if index.is_in_bounds(self.len(), &self.owner).is_ok() {
            // SAFETY: index.is_in_bounds checks that the index is in bounds, and ranges are well
            // ordered
            Some(unsafe { self.get_unchecked_mut(index) })
        } else {
            None
        }
    }
}

#[cfg(feature = "unique-types")]
impl<T, O: ?Sized + UniqueToken> UtVec<T, O> {
    /// Check if a given index is in bounds, if so return a [`UtIndex`] version of that index
    pub fn is_in_bounds(&self, i: usize) -> Option<UtIndex<O>> {
        self.indices().nth(i)
    }

    /// An iterator over all valid indices in this vector
    pub fn indices(&self) -> Indices<O> {
        Indices {
            token: self.owner.token(),
            start: 0,
            end: self.len(),
        }
    }
}

#[cfg(feature = "unique-types")]
/// An iterator over all indices in a [`UtVec`]
pub struct Indices<O: ?Sized + UniqueToken> {
    token: O::Token,
    start: usize,
    end: usize,
}

#[cfg(feature = "unique-types")]
impl<O: UniqueToken + ?Sized> ExactSizeIterator for Indices<O> {}
#[cfg(feature = "unique-types")]
impl<O: UniqueToken + ?Sized> core::iter::FusedIterator for Indices<O> {}
#[cfg(feature = "unique-types")]
impl<O: UniqueToken + ?Sized> Iterator for Indices<O> {
    type Item = UtIndex<O>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            let index = self.start;
            self.start += 1;
            Some(UtIndex {
                token: self.token,
                index,
            })
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let (start, ovf) = self.start.overflowing_add(n);
        if ovf || start >= self.end {
            self.start = self.end;
            None
        } else {
            self.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.start;
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.len()
    }
}

#[cfg(feature = "unique-types")]
impl<O: UniqueToken + ?Sized> DoubleEndedIterator for Indices<O> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            let index = self.end.wrapping_sub(1);
            self.end = index;
            Some(UtIndex {
                token: self.token,
                index,
            })
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let (end, ovf) = self.end.overflowing_sub(n);
        if ovf || self.start >= end {
            self.start = self.end;
            None
        } else {
            self.next_back()
        }
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

impl<T, O: ?Sized, I: UtVecIndex<O>> ops::Index<I> for UtVec<T, O> {
    type Output = GetOutputType<I, O, T>;

    fn index(&self, index: I) -> &Self::Output {
        match index.is_in_bounds(self.len(), &self.owner) {
            Err(err) => handle!(err),
            // SAFETY: is_in_bounds ensures that the index is in bounds, and ranges are well ordered
            Ok(()) => unsafe { self.get_unchecked(index) },
        }
    }
}

impl<T, O: ?Sized, I: UtVecIndex<O>> ops::IndexMut<I> for UtVec<T, O> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        match index.is_in_bounds(self.len(), &self.owner) {
            Err(err) => handle!(err),
            // SAFETY: is_in_bounds ensures that the index is in bounds, and ranges are well ordered
            Ok(()) => unsafe { self.get_unchecked_mut(index) },
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

macro_rules! handle {
    ($err:expr) => {{
        #[cold]
        #[inline(never)]
        fn not_owned() -> ! {
            panic!("Index not owned by `UtVec`")
        }

        #[cold]
        #[inline(never)]
        fn not_in_bounds_exc(index: usize, len: usize) -> ! {
            panic!("Index out of bounds (index > length), index: {index}, length: {len}")
        }

        #[cold]
        #[inline(never)]
        fn not_in_bounds_inc(index: usize, len: usize) -> ! {
            panic!("Index out of bounds (index >= length), index: {index}, length: {len}")
        }

        #[cold]
        #[inline(never)]
        fn bad_order(start: usize, end: usize) -> ! {
            panic!("Range bounds out of order (start > end), start: {start}, end: {end}")
        }

        match $err {
            $crate::IndexError::NotOwned => not_owned(),
            $crate::IndexError::NotInBounds {
                index,
                len,
                is_inclusive: false,
            } => not_in_bounds_exc(index, len),
            $crate::IndexError::NotInBounds {
                index,
                len,
                is_inclusive: true,
            } => not_in_bounds_inc(index, len),
            $crate::IndexError::OutOfOrder { start, end } => bad_order(start, end),
        }
    }};
}
use handle;

impl IndexError {
    /// Panics with the appropriate error message
    #[inline(always)]
    pub fn handle<T>(self) -> ! {
        handle!(self)
    }
}

/// An output type specifier to [`UtVecIndex`]
pub trait OutputKind {
    /// The output type of [`UtVecIndex::offset_slice`]
    type Output<T>: ?Sized;
}

type GetOutputType<I, O, T> = <<I as UtVecIndex<O>>::OutputKind as OutputKind>::Output<T>;

/// An [`OutputKind`] where `Output<T> = T`
pub struct Element;
/// An [`OutputKind`] where `Output<T> = [T]`
pub struct Slice;

impl OutputKind for Element {
    type Output<T> = T;
}

impl OutputKind for Slice {
    type Output<T> = [T];
}

/// Any type which can be used to index into a [`UtVec`]
///
/// This includes, usize, ranges over usize, [`UtIndex`], and ranges over [`UtIndex`]
pub trait UtVecIndex<O: ?Sized>: Seal {
    /// What kind of output does [`UtVecIndex::offset_slice`] return
    type OutputKind: OutputKind;

    /// Check if this index is in bounds of the [`UtVec`] that owns `O`
    fn is_in_bounds(&self, len: usize, owner: &O) -> Result<(), IndexError>;

    /// Indexes into the slice without checking if self is in bounds
    ///
    /// # Safety
    ///
    /// `slice` must in a single allocated for it's entire length
    /// `is_in_bounds` must return Ok when passed the length of the slice and
    /// the owner associated with the slice
    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>>;
}

/// Any type which can be used to index into a [`UtVec`]
///
/// This includes, usize, ranges over usize, [`UtIndex`], and ranges over [`UtIndex`]
pub trait UtVecElementIndex<O: ?Sized>: UtVecIndex<O, OutputKind = Element> {
    /// Get the underlying index that this value represents
    fn get_index(&self) -> usize;
}

impl Seal for ops::RangeFull {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeFull {
    type OutputKind = Slice;

    fn is_in_bounds(&self, _len: usize, _owner: &O) -> Result<(), IndexError> {
        Ok(())
    }

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        _owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        slice
    }
}

impl Seal for usize {}
impl<O: ?Sized> UtVecIndex<O> for usize {
    type OutputKind = Element;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        _owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: is_in_bounds checks that self is in bounds of the slice length
        // and the slice is in a single allocation for it's entire, so offseting
        // somewhere inside that length if fine
        unsafe { NonNull::new_unchecked(slice.as_ptr().cast::<T>().add(self)) }
    }
}

impl<O: ?Sized> UtVecElementIndex<O> for usize {
    #[inline]
    fn get_index(&self) -> usize {
        *self
    }
}

impl Seal for ops::RangeTo<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeTo<usize> {
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        _owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
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
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        _owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
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
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        _owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
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
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            let slice = (..self.end).offset_slice(slice, owner);
            (self.start..).offset_slice(slice, owner)
        }
    }
}

impl Seal for ops::RangeInclusive<usize> {}
impl<O: ?Sized> UtVecIndex<O> for ops::RangeInclusive<usize> {
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: the input is nonnull, so the output must be nonnull
        unsafe {
            let slice = (..=*self.end()).offset_slice(slice, owner);
            (*self.start()..).offset_slice(slice, owner)
        }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for UtIndex<O> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for UtIndex<O> {
    type OutputKind = Element;

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { self.index.offset_slice(slice, owner) }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecElementIndex<O> for UtIndex<O> {
    #[inline]
    fn get_index(&self) -> usize {
        self.index
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for ops::RangeTo<UtIndex<O>> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeTo<UtIndex<O>> {
    type OutputKind = Slice;

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.end.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (..self.end.index).offset_slice(slice, owner) }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for ops::RangeToInclusive<UtIndex<O>> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeToInclusive<UtIndex<O>> {
    type OutputKind = Slice;

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.end.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (..=self.end.index).offset_slice(slice, owner) }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for ops::RangeFrom<UtIndex<O>> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeFrom<UtIndex<O>> {
    type OutputKind = Slice;

    fn is_in_bounds(&self, _len: usize, owner: &O) -> Result<(), IndexError> {
        if owner.owns(&self.start.token) {
            Ok(())
        } else {
            Err(IndexError::NotOwned)
        }
    }

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start.index..).offset_slice(slice, owner) }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for ops::Range<UtIndex<O>> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::Range<UtIndex<O>> {
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start.index..self.end.index).offset_slice(slice, owner) }
    }
}

#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> Seal for ops::RangeInclusive<UtIndex<O>> {}
#[cfg(feature = "unique-types")]
impl<O: ?Sized + UniqueToken> UtVecIndex<O> for ops::RangeInclusive<UtIndex<O>> {
    type OutputKind = Slice;

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

    unsafe fn offset_slice<T>(
        self,
        slice: NonNull<[T]>,
        owner: &O,
    ) -> NonNull<GetOutputType<Self, O, T>> {
        // SAFETY: if the owner owns this index, then it is guaranteed to be in bounds
        unsafe { (self.start().index..=self.end().index).offset_slice(slice, owner) }
    }
}
