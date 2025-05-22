//! An implementation of dense arenas with a lot of knobs to tweak
//!
//! see [`GenericDenseArena`] for details

use core::ops;

use alloc::vec::Vec;

use crate::{
    dense_tracker::{self, GenericDenseTracker},
    generation::{DefaultGeneration, Generation},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

/// [`GenericDenseArena`] is the canonical implementation of how to use [`GenericDenseTracker`]
///
/// It pairs a [`GenericDenseTracker`] with a [`Vec<T>`]
pub struct GenericDenseArena<
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    values: Vec<T>,
    tracker: GenericDenseTracker<O, G, I>,
}

/// A vacant slot into a [`GenericDenseArena`]
pub struct VacantSlot<
    'a,
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    slot: dense_tracker::VacantSlot<'a, O, G, I>,
    vec: &'a mut Vec<T>,
}

impl<T, G: Generation, I: InternalIndex> GenericDenseArena<T, (), G, I> {
    /// Create a new [`GenericDenseArena`]
    pub const fn new() -> Self {
        Self {
            values: Vec::new(),
            tracker: GenericDenseTracker::new(),
        }
    }
}

impl<T, G: Generation, I: InternalIndex> Default for GenericDenseArena<T, (), G, I> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "unique-types")]
impl<T, O, G: Generation, I: InternalIndex> GenericDenseArena<T, O, G, I> {
    /// Create a new [`GenericDenseArena`] with the given owner
    pub const fn with_owner(owner: O) -> Self
    where
        O: unique_types::UniqueToken,
    {
        Self {
            values: Vec::new(),
            tracker: GenericDenseTracker::with_owner(owner),
        }
    }

    /// Get the owner of this type's keys
    pub fn owner(&self) -> &O {
        self.tracker.owner()
    }
}

impl<T, O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, T, O, G, I> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        self.slot.key()
    }

    /// Insert an element into this slot
    pub fn insert(self, value: T) {
        let index = self.slot.position();
        debug_assert_eq!(index, self.vec.len());

        // SAFETY: [`GenericDenseArena::vacant_slot`] ensures that the vector has
        // enough capacity for this write
        unsafe {
            self.vec.as_mut_ptr().add(index).write(value);
            self.vec.set_len(index + 1);
        }

        self.slot.insert()
    }
}

impl<T, O, G: Generation, I: InternalIndex> GenericDenseArena<T, O, G, I>
where
    O: core::fmt::Debug,
{
    /// Access a vacant slot in the arena
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.values.len() == self.values.capacity() {
            self.values.reserve(1);
        }

        VacantSlot {
            slot: self.tracker.vacant_slot(self.values.len()),
            vec: &mut self.values,
        }
    }

    /// Insert a new value into a [`GenericDenseArena`]
    pub fn insert<K: ArenaIndex<O, G>>(&mut self, value: T) -> K {
        self.insert_with(move |_| value)
    }

    /// Insert a new value that depends on the key into a [`GenericDenseArena`]
    pub fn insert_with<K: ArenaIndex<O, G>>(&mut self, value: impl FnOnce(K) -> T) -> K {
        let slot = self.vacant_slot();
        let key = slot.key();
        slot.insert(value(key));
        key
    }

    /// Get a reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<&T> {
        let index = self.tracker.get(key)?;
        // SAFETY: the tracker ensures that index is in bounds
        Some(unsafe { self.values.get_unchecked(index) })
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn get_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<&mut T> {
        let index = self.tracker.get(key)?;
        // SAFETY: the tracker ensures that index is in bounds
        Some(unsafe { self.values.get_unchecked_mut(index) })
    }

    /// Get a reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and must have the correct generation
    ///
    /// i.e. [`GenericDenseArena::get`] would have returned [`Some`]
    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> &T {
        // SAFETY: the caller ensures that the key is valid
        let index = unsafe { self.tracker.get_unchecked(key) };
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.values.get_unchecked(index) }
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and must have the correct generation
    ///
    /// i.e. [`GenericDenseArena::get_mut`] would have returned [`Some`]
    #[inline]
    pub unsafe fn get_unchecked_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> &mut T {
        // SAFETY: the caller ensures that the key is valid
        let index = unsafe { self.tracker.get_unchecked(key) };
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.values.get_unchecked_mut(index) }
    }

    unsafe fn remove_at(&mut self, index: usize) -> T {
        if index >= self.values.len() {
            // SAFETY: all callers ensure that the index is in bounds
            unsafe { core::hint::unreachable_unchecked() }
        }

        self.values.swap_remove(index)
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<T> {
        let index = self.tracker.try_remove(key)?;
        // SAFETY: the tracker ensures that index is in bounds
        Some(unsafe { self.remove_at(index) })
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = self.tracker.remove(key);
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.remove_at(index) }
    }

    /// Remove the element associated with the key without checking
    /// if the key is invalid or out of bounds
    ///
    /// # Safety
    ///
    /// They key must be in bounds, and point to a filled slot
    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        // SAFETY: the caller ensures that the key is valid
        let index = unsafe { self.tracker.remove_unchecked(key) };
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.remove_at(index) }
    }

    /// The slice of values in this [`GenericDenseArena`]
    #[inline]
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The mutable slice of values in this [`GenericDenseArena`]
    #[inline]
    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    /// The [`GenericDenseTracker`] that this [`GenericDenseArena`] uses
    #[inline]
    pub const fn tracker(&self) -> &GenericDenseTracker<O, G, I> {
        &self.tracker
    }

    /// The mutable slice of values in this [`GenericDenseArena`]
    /// and the [`GenericDenseTracker`] that this [`GenericDenseArena`] uses
    ///
    /// This method is to work around limitations in Rust's borrow checker
    #[inline]
    pub fn values_mut_and_tracker(&mut self) -> (&mut [T], &GenericDenseTracker<O, G, I>) {
        (&mut self.values, &self.tracker)
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::Index<K>
    for GenericDenseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let index = self.tracker.at(index).to_usize();
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.values.get_unchecked(index) }
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::IndexMut<K>
    for GenericDenseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let index = self.tracker.at(index).to_usize();
        // SAFETY: the tracker ensures that index is in bounds
        unsafe { self.values.get_unchecked_mut(index) }
    }
}

#[cfg(test)]
mod tests {
    use super::GenericDenseArena;

    #[test]
    fn basic() {
        let mut arena = GenericDenseArena::<u32, (), crate::generation::g8>::new();

        let a: crate::key::ArenaKey<usize, _> = arena.insert(0);

        assert_eq!(arena[a], 0);

        arena.remove(a);

        let b: crate::key::ArenaKey<usize, _> = arena.insert(10);

        assert_eq!(a.index(), b.index());
        assert_eq!(arena[b], 10);
        assert_eq!(arena.get(a), None);

        arena.remove(b);

        for _ in 0..126 {
            let a: crate::key::ArenaKey<usize, _> = arena.insert(0);
            assert_eq!(a.index(), b.index());

            assert_eq!(arena[a], 0);

            arena.remove(a);
        }

        // at this point we have exhausted the first slot, so it will never be used again

        let a: crate::key::ArenaKey<usize, _> = arena.insert(0);
        assert_ne!(a.index(), b.index());
    }
}
