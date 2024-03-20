//! dense tracker is a type to allow you to build your own dense arenas
//!
//! This is used in [`GenericDenseArena`](crate::generic_dense::GenericDenseArena) to
//! track which keys point to which elements.
//!
//! This [`GenericDenseTracker`] should be associated with an array (or set or arrays).
//!
//! * Each time you call [`VacantSlot::insert`], you must push an element into the array(s)
//! * Each time you call [`GenericDenseTracker::remove`] (or it's variants), successfully
//!  you must [`Vec::swap_remove`] the corresponding element out of the array(s)
//!
//! If you do these two things, then all indices in the
//! [`GenericDenseTracker`](crate::generic_dense::GenericDenseArena) are guaranteed
//! to be correct indices into you array(s).
//!
//! This allows you to build up your own dense arenas. For example,
//! [`GenericDenseArena`](crate::generic_dense::GenericDenseArena) stores all elemnts as an
//! [AoS](https://en.wikipedia.org/wiki/AoS_and_SoA) and you could store them as a SoA instead
//! to improve iteration performance of some fields.

use core::marker::PhantomData;

use alloc::vec::Vec;

use crate::{
    generation::{DefaultGeneration, Generation},
    generic_sparse::{self as sparse, GenericSparseArena},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

/// A dense tracker keeps track of which keys point to which indices
///
/// This structure should be paired with an array of elements that store the actual data
pub struct GenericDenseTracker<
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    keys: Vec<I>,
    index: GenericSparseArena<I, O, G, I>,
}

/// A vacant slot in a [`GenericDenseTracker`], created by [`GenericDenseTracker::vacant_slot`]
pub struct VacantSlot<
    'a,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    sparse: sparse::VacantSlot<'a, I, O, G, I>,
    index_rev: &'a mut Vec<I>,
}

impl<G: Generation, I: InternalIndex> GenericDenseTracker<(), G, I> {
    /// Create a new [`GenericDenseTracker`]
    pub const fn new() -> Self {
        Self {
            keys: Vec::new(),
            index: GenericSparseArena::new(),
        }
    }
}

impl<G: Generation, I: InternalIndex> Default for GenericDenseTracker<(), G, I> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "unique-types")]
impl<O, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I> {
    /// Create a new [`GenericDenseTracker`] with the given owner    
    pub const fn with_owner(owner: O) -> Self
    where
        O: unique_types::UniqueToken,
    {
        Self {
            keys: Vec::new(),
            index: GenericSparseArena::with_owner(owner),
        }
    }

    /// Get the owner of this type's keys
    pub fn owner(&self) -> &O {
        self.index.owner()
    }
}

impl<O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, O, G, I> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        self.sparse.key()
    }

    /// Get the position of the slot into the associated array once it is filled
    pub fn position(&self) -> usize {
        self.index_rev.len()
    }

    /// Insert an element into this slot
    ///
    /// This should be called along side inserting the element at
    /// `self.position()` in the associated array
    pub fn insert(self) {
        let len = self.position();
        self.index_rev.push(I::from_usize(self.sparse.key()));
        self.sparse.insert(I::from_usize(len))
    }
}

impl<O: ?Sized, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I> {
    /// Access a vacant slot in the arena
    pub fn vacant_slot(&mut self, len: usize) -> VacantSlot<'_, O, G, I> {
        assert_eq!(self.keys.len(), len);
        VacantSlot {
            sparse: self.index.vacant_slot(),
            index_rev: &mut self.keys,
        }
    }

    /// The number of elements in the arena
    #[inline]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns true if there are no elements in the arena
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Get the index into the array associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<usize> {
        self.index.get(key).copied().map(I::to_usize)
    }

    /// Get the index into the array associated with the key
    ///
    /// # Panics
    ///
    /// If the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn at<K: ArenaIndex<O, G>>(&self, key: K) -> usize {
        self.index[key].to_usize()
    }

    /// Get the index into the array associated with the key without checking
    /// if it's in bounds or has the correct generation
    ///
    /// # Safety
    ///
    /// The key must be in bounds and must have the correct generation
    ///
    /// i.e. [`GenericDenseTracker::get`] would have returned [`Some`]
    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> usize {
        // SAFETY: the caller ensures that the key is valid
        unsafe { *self.index.get_unchecked(key) }.to_usize()
    }

    /// Get the key associated with an index into the arena
    ///
    /// NOTE: This is NOT the index into the associated array, but an index into
    /// to slot list of the arena.
    ///
    /// Returns [`None`] if the index points to an empty slot, or is out of bounds
    #[inline]
    pub fn try_key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> Option<K> {
        self.index.try_key_of(index)
    }

    /// Get the key associated with an index into the arena
    ///
    /// NOTE: This is NOT the index into the associated array, but an index into
    /// to slot list of the arena.
    ///
    /// # Panics
    ///
    /// If the index points to an empty slot, or is out of bounds
    #[inline]
    pub fn key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        self.index.key_of(index)
    }

    /// Get the key associated with an index into the arena
    ///
    /// NOTE: This is NOT the index into the associated array, but an index into
    /// to slot list of the arena.
    ///
    /// # Safety
    ///
    /// The index must be in bounds and must point to a filled slot
    #[inline]
    pub unsafe fn key_of_unchecked<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        // SAFETY: the caller ensures that the index is in bounds and points to a filled slot
        unsafe { self.index.key_of_unchecked(index) }
    }

    fn remove_at(&mut self, index_fwd: I) -> usize {
        if self.keys.is_empty() {
            debug_assert!(false);
            // SAFETY: all callers ensure that the arena isn't empty
            unsafe { core::hint::unreachable_unchecked() }
        }
        if index_fwd.to_usize() >= self.keys.len() {
            debug_assert!(false, "{index_fwd:?} >= {}", self.keys.len());
            // SAFETY: all callers ensure that the index was obtained from self.index_fwd
            // which only contains valid indices
            unsafe { core::hint::unreachable_unchecked() }
        }

        self.keys.swap_remove(index_fwd.to_usize());

        // If we are removing the end of the list, then we shouldn't do any more updates
        if let Some(&key_of_end) = self.keys.get(index_fwd.to_usize()) {
            // we need to update the forward mapping to show that the end is now pointing to index_fwd
            // SAFETY: index_end_rev was obtained from self.index_rev, which only contains
            // valid keys into self.index_fwd
            unsafe { *self.index.get_unchecked_mut(key_of_end.to_usize()) = index_fwd }
        }

        index_fwd.to_usize()
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<usize> {
        let index_fwd = self.index.try_remove(key)?;
        Some(self.remove_at(index_fwd))
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> usize {
        let index_fwd = self.index.remove(key);
        self.remove_at(index_fwd)
    }

    /// Remove the element associated with the key without checking
    /// if the key is invalid or out of bounds
    ///
    /// # Safety
    ///
    /// They key must be in bounds, and point to a filled slot
    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> usize {
        // SAFETY: caller ensures that the key is in bounds and points to a filled slot
        let index_fwd = unsafe { self.index.remove_unchecked(key) };
        self.remove_at(index_fwd)
    }

    /// Get an iterator over all the keys in the arena
    ///
    /// This iterator will yield exactly `self.len` elements
    ///
    /// and [`Keys::next`] has O(1) performance
    pub fn keys<K: ArenaIndex<O, G>>(&self) -> Keys<'_, K, O, G, I> {
        Keys {
            index_rev: self.keys.iter(),
            index_fwd: &self.index,
            _key: PhantomData,
        }
    }
}

/// An iterator over the keys of a [`GenericDenseTracker`], created from
/// [`GenericDenseTracker::keys`]
pub struct Keys<'a, K, O: ?Sized, G: Generation, I: InternalIndex> {
    index_rev: core::slice::Iter<'a, I>,
    index_fwd: &'a GenericSparseArena<I, O, G, I>,
    _key: PhantomData<fn() -> K>,
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex> ExactSizeIterator
    for Keys<'_, K, O, G, I>
{
}
impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex> Iterator
    for Keys<'_, K, O, G, I>
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        let index_rev = *self.index_rev.next()?;
        // SAFETY: all keys in self.index_rev are valid and in bounds
        Some(unsafe { self.index_fwd.key_of_unchecked(index_rev.to_usize()) })
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let index_rev = *self.index_rev.nth(n)?;
        // SAFETY: all keys in self.index_rev are valid and in bounds
        Some(unsafe { self.index_fwd.key_of_unchecked(index_rev.to_usize()) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.index_rev.size_hint()
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex> DoubleEndedIterator
    for Keys<'_, K, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let index_rev = *self.index_rev.next_back()?;
        // SAFETY: all keys in self.index_rev are valid and in bounds
        Some(unsafe { self.index_fwd.key_of_unchecked(index_rev.to_usize()) })
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let index_rev = *self.index_rev.nth_back(n)?;
        // SAFETY: all keys in self.index_rev are valid and in bounds
        Some(unsafe { self.index_fwd.key_of_unchecked(index_rev.to_usize()) })
    }
}
