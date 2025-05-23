//! This is a reimplementation of the `slotmap` crate which is based off of
//! [`GenericSparseArena`]
//!
//! It should retain all the same performance and memory characteristics as `SlotMap`

use crate::{
    generation::gw32,
    generic_sparse::{self as sparse, GenericSparseArena},
};

/// The key type for [`SlotMap`]
pub type ArenaKey = crate::key::ArenaKey<u32, gw32>;

/// see [`GenericSparseArena`]
///
/// [`SlotMap`] is instantiated as `GenericSparseArena<T, (), gw32, u32>` and
/// has an extra length field for compatibility with the `slotmap` crate
pub struct SlotMap<T> {
    len: u32,
    arena: GenericSparseArena<T, (), gw32, u32>,
}

/// a vacant slot into the [`SlotMap`], created via [`SlotMap::vacant_slot`]
pub struct VacantSlot<'a, T> {
    len: &'a mut u32,
    slot: sparse::VacantSlot<'a, T, (), gw32>,
}

impl<T> VacantSlot<'_, T> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key(&self) -> ArenaKey {
        self.slot.key()
    }

    /// Insert an element into this slot
    pub fn insert(self, value: T) {
        self.slot.insert(value);
        *self.len += 1;
    }
}

impl<T> SlotMap<T> {
    /// Create a new [`SlotMap`]
    pub const fn new() -> Self {
        Self {
            len: 0,
            arena: GenericSparseArena::new(),
        }
    }

    /// Get the number of elements in the [`SlotMap`]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns true if there are no elements in the [`SlotMap`]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert a new value into a [`SlotMap`]
    pub fn insert(&mut self, value: T) -> ArenaKey {
        self.len += 1;
        self.arena.insert(value)
    }

    /// Insert a new value that depends on the key into a [`SlotMap`]
    pub fn insert_with(&mut self, value: impl FnOnce(ArenaKey) -> T) -> ArenaKey {
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
    pub fn get(&self, key: ArenaKey) -> Option<&T> {
        self.arena.get(key)
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or if the slot is empty)
    pub fn get_mut(&mut self, key: ArenaKey) -> Option<&mut T> {
        self.arena.get_mut(key)
    }

    /// Get a reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and the slot must be filled
    ///
    /// i.e. [`SlotMap::get`] would have returned [`Some`]
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
    /// i.e. [`SlotMap::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        // SAFETY: the caller ensures that this is correct
        unsafe { self.arena.get_unchecked_mut(key) }
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    pub fn try_remove(&mut self, key: ArenaKey) -> Option<T> {
        let value = self.arena.try_remove(key);
        self.len -= value.is_some() as u32;
        value
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    pub fn remove(&mut self, key: ArenaKey) -> T {
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
    pub fn values(&self) -> sparse::Values<'_, T, gw32, u32> {
        self.arena.values()
    }

    /// Get an iterator over the mut references to elements of this arena
    pub fn values_mut(&mut self) -> sparse::ValuesMut<'_, T, gw32, u32> {
        self.arena.values_mut()
    }

    /// Get an iterator over the keys of this arena
    pub fn keys(&self) -> sparse::Keys<'_, ArenaKey, T, (), gw32, u32> {
        self.arena.keys()
    }

    /// Get an iterator over the keys and references to elements of this arena
    pub fn iter(&self) -> sparse::Iter<'_, ArenaKey, T, (), gw32, u32> {
        self.arena.iter()
    }

    /// Get an iterator over the keys and mut references to elements of this arena
    pub fn iter_mut(&mut self) -> sparse::IterMut<'_, ArenaKey, T, (), gw32, u32> {
        self.arena.iter_mut()
    }
}

impl<T> Default for SlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::ops::Index<ArenaKey> for SlotMap<T> {
    type Output = T;

    fn index(&self, index: ArenaKey) -> &Self::Output {
        &self.arena[index]
    }
}

impl<T> core::ops::IndexMut<ArenaKey> for SlotMap<T> {
    fn index_mut(&mut self, index: ArenaKey) -> &mut Self::Output {
        &mut self.arena[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut s = SlotMap::new();
        let a = s.insert(10);
        let b = s.insert(20);
        assert_eq!(s[a], 10);
        assert_eq!(s[b], 20);
        s.remove(a);
        assert_eq!(s[b], 20);
    }
}
