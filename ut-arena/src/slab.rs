//! This is a reimplementation of the `slab` crate which is based off of
//! [`GenericSparseArena`]
//!
//! It should retain all the same performance and memory characteristics as `slab`

use crate::{
    generation::NoGeneration,
    generic_sparse::{self as sparse, GenericSparseArena},
};

/// see [`GenericSparseArena`]
///
/// [`Slab`] is instantiated as `GenericSparseArena<T, (), NoGeneration, usize>` and
/// has an extra length field for compatibility with the `slab` crate
pub struct Slab<T> {
    len: usize,
    arena: GenericSparseArena<T, (), NoGeneration, usize>,
}

/// a vacant slot into the [`Slab`], created via [`Slab::vacant_slot`]
pub struct VacantSlot<'a, T> {
    len: &'a mut usize,
    slot: sparse::VacantSlot<'a, T, (), NoGeneration, usize>,
}

impl<T> VacantSlot<'_, T> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key(&self) -> usize {
        self.slot.key()
    }

    /// Insert an element into this slot
    pub fn insert(self, value: T) {
        self.slot.insert(value);
        *self.len += 1;
    }
}

impl<T> Slab<T> {
    /// Create a new [`Slab`]
    pub const fn new() -> Self {
        Self {
            len: 0,
            arena: GenericSparseArena::new(),
        }
    }

    /// Get the number of elements in the [`Slab`]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if there are no elements in the [`Slab`]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert a new value into a [`Slab`]
    pub fn insert(&mut self, value: T) -> usize {
        self.len += 1;
        self.arena.insert(value)
    }

    /// Insert a new value that depends on the key into a [`Slab`]
    pub fn insert_with(&mut self, value: impl FnOnce(usize) -> T) -> usize {
        self.len += 1;
        self.arena.insert_with(value)
    }

    /// Access a vacant slot in the arena
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T> {
        VacantSlot {
            len: &mut self.len,
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
    /// i.e. [`Slab::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked(&self, key: usize) -> &T {
        // SAFETY: the caller ensures that this is correct
        unsafe { self.arena.get_unchecked(key) }
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and the slot must be filled
    ///
    /// i.e. [`Slab::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        // SAFETY: the caller ensures that this is correct
        unsafe { self.arena.get_unchecked_mut(key) }
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    pub fn try_remove(&mut self, key: usize) -> Option<T> {
        let value = self.arena.try_remove(key);
        self.len -= value.is_some() as usize;
        value
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    pub fn remove(&mut self, key: usize) -> T {
        let value = self.arena.remove(key);
        self.len -= 1;
        value
    }

    /// Remove the element associated with the key without checking
    /// if the key is invalid or out of bounds
    ///
    /// # Safety
    ///
    /// They key must be in bounds, and point to a filled slot
    pub unsafe fn remove_unchecked(&mut self, key: usize) -> T {
        self.len -= 1;
        // SAFETY: the caller ensures that the key is in bounds and points to a filled slot
        unsafe { self.arena.remove_unchecked(key) }
    }

    /// Get an iterator over the references to elements of this arena
    pub fn values(&self) -> sparse::Values<'_, T, NoGeneration, usize> {
        self.arena.values()
    }

    /// Get an iterator over the mut references to elements of this arena
    pub fn values_mut(&mut self) -> sparse::ValuesMut<'_, T, NoGeneration, usize> {
        self.arena.values_mut()
    }

    /// Get an iterator over the keys of this arena
    pub fn keys(&self) -> sparse::Keys<'_, usize, T, (), NoGeneration, usize> {
        self.arena.keys()
    }

    /// Get an iterator over the keys and references to elements of this arena
    pub fn iter(&self) -> sparse::Iter<'_, usize, T, (), NoGeneration, usize> {
        self.arena.iter()
    }

    /// Get an iterator over the keys and mut references to elements of this arena
    pub fn iter_mut(&mut self) -> sparse::IterMut<'_, usize, T, (), NoGeneration, usize> {
        self.arena.iter_mut()
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::ops::Index<usize> for Slab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arena[index]
    }
}

impl<T> core::ops::IndexMut<usize> for Slab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.arena[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = Slab::new();
        s.insert(10);
        s.insert(20);
        assert_eq!(s[0], 10);
        assert_eq!(s[1], 20);
        s.remove(0);
        assert_eq!(s[1], 20);
    }
}
