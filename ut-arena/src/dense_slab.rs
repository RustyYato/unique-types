//! This is an extension of the `slab` crate which is based off of
//! [`GenericDenseArena`]

use crate::{
    generation::NoGeneration,
    generic_dense::{self as dense, GenericDenseArena},
};

/// see [`GenericDenseArena`]
///
/// [`DenseSlab`] is instantiated as `GenericDenseArena<T, (), NoGeneration, usize>`
pub struct DenseSlab<T> {
    /// The underlying generic arena type
    pub arena: GenericDenseArena<T, (), NoGeneration, usize>,
}

/// a vacant slot into the [`DenseSlab`], created via [`DenseSlab::vacant_slot`]
pub struct VacantSlot<'a, T> {
    /// The underlying generic vacant slot type
    pub slot: dense::VacantSlot<'a, T, (), NoGeneration, usize>,
}

impl<T> VacantSlot<'_, T> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key(&self) -> usize {
        self.slot.key()
    }

    /// Insert an element into this slot
    pub fn insert(self, value: T) {
        self.slot.insert(value);
    }
}

impl<T> DenseSlab<T> {
    /// Create a new [`DenseSlab`]
    pub const fn new() -> Self {
        Self {
            arena: GenericDenseArena::new(),
        }
    }

    /// Get the number of elements in the [`DenseSlab`]
    pub const fn len(&self) -> usize {
        self.arena.tracker().len()
    }

    /// Returns true if there are no elements in the [`DenseSlab`]
    pub const fn is_empty(&self) -> bool {
        self.arena.tracker().is_empty()
    }

    /// Insert a new value into a [`DenseSlab`]
    pub fn insert(&mut self, value: T) -> usize {
        self.arena.insert(value)
    }

    /// Insert a new value that depends on the key into a [`DenseSlab`]
    pub fn insert_with(&mut self, value: impl FnOnce(usize) -> T) -> usize {
        self.arena.insert_with(value)
    }

    /// Access a vacant slot in the arena
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T> {
        VacantSlot {
            slot: self.arena.vacant_slot(),
        }
    }

    /// Get a reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or if the slot is empty)
    pub fn get(&self, key: usize) -> Option<&T> {
        self.arena.get(key)
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or if the slot is empty)
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        self.arena.get_mut(key)
    }

    /// Get a reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and the slot must be filled
    ///
    /// i.e. [`DenseSlab::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked(&self, key: usize) -> &T {
        // SAFETY: the caller ensures that this is safe
        unsafe { self.arena.get_unchecked(key) }
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and the slot must be filled
    ///
    /// i.e. [`DenseSlab::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        // SAFETY: the caller ensures that this is safe
        unsafe { self.arena.get_unchecked_mut(key) }
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    pub fn try_remove(&mut self, key: usize) -> Option<T> {
        self.arena.try_remove(key)
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    pub fn remove(&mut self, key: usize) -> T {
        self.arena.remove(key)
    }

    /// Remove the element associated with the key without checking
    /// if the key is invalid or out of bounds
    ///
    /// # Safety
    ///
    /// They key must be in bounds, and point to a filled slot
    pub unsafe fn remove_unchecked(&mut self, key: usize) -> T {
        // SAFETY: the caller ensures that the key is in bounds and points to a filled slot
        unsafe { self.arena.remove_unchecked(key) }
    }

    /// An unordered list of values in the slab
    pub fn values(&self) -> &[T] {
        self.arena.values()
    }

    /// An mutable unordered list of values in the slab
    pub fn values_mut(&mut self) -> &mut [T] {
        self.arena.values_mut()
    }

    /// An iterator over all the keys in the slab
    pub fn keys(&self) -> Keys<'_> {
        Keys {
            keys: self.arena.tracker().keys(),
        }
    }

    /// The mutable slice of values in this [`DenseSlab`]
    /// and the [`GenericDenseTracker`](crate::dense_tracker::GenericDenseTracker)
    /// that this [`DenseSlab`] uses
    ///
    /// This method is to work around limitations in Rust's borrow checker
    pub fn keys_and_values_mut(&mut self) -> (Keys<'_>, &mut [T]) {
        let (values, tracker) = self.arena.values_mut_and_tracker();
        (
            Keys {
                keys: tracker.keys(),
            },
            values,
        )
    }
}

impl<T> Default for DenseSlab<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::ops::Index<usize> for DenseSlab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arena[index]
    }
}

impl<T> core::ops::IndexMut<usize> for DenseSlab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.arena[index]
    }
}

/// An iterator over the keys in a [`DenseSlab`]
pub struct Keys<'a> {
    keys: crate::dense_tracker::Keys<'a, usize, (), NoGeneration, usize>,
}

impl ExactSizeIterator for Keys<'_> {}
impl Iterator for Keys<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.keys.nth(n)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl DoubleEndedIterator for Keys<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.keys.next_back()
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.keys.nth_back(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = DenseSlab::new();
        s.insert(10);
        s.insert(20);
        assert_eq!(s[0], 10);
        assert_eq!(s[1], 20);
        s.remove(0);
        assert_eq!(s[1], 20);
    }
}
