//! This is a reimplementation of the `slotmap` crate which is based off of
//! [`GenericDenseArena`]
//!
//! It should retain all the same performance and memory characteristics as [`DenseSlotMap`]

use crate::{
    generation::gw32,
    generic_dense::{self as dense, GenericDenseArena},
};

/// The key type for [`DenseSlotMap`]
pub type ArenaKey = crate::key::ArenaKey<u32, gw32>;

/// see [`GenericDenseArena`]
///
/// [`DenseSlotMap`] is instantiated as `GenericDenseArena<T, (), gw32, u32>`
pub struct DenseSlotMap<T> {
    /// the generic arena this [`DenseSlotMap`] is based on
    pub arena: GenericDenseArena<T, (), gw32, u32>,
}

/// a vacant slot into the [`DenseSlotMap`], created via [`DenseSlotMap::vacant_slot`]
pub struct VacantSlot<'a, T> {
    /// the generic slot that this [`DenseSlotMap`]'s vacant slot is based on
    pub slot: dense::VacantSlot<'a, T, (), gw32, u32>,
}

impl<T> VacantSlot<'_, T> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key(&self) -> ArenaKey {
        self.slot.key()
    }

    /// Insert an element into this slot
    pub fn insert(self, value: T) {
        self.slot.insert(value);
    }

    /// try to insert an element into the slot by writing directly into it. If initializing
    /// the value fails, then the vacant slot is returned
    #[cfg(feature = "init")]
    pub fn try_init<Init>(self, init: Init) -> Result<(), (Init::Error, Self)>
    where
        Init: init::Initializer<T>,
    {
        match self.slot.try_init(init) {
            Ok(()) => Ok(()),
            Err((err, slot)) => Err((err, Self { slot })),
        }
    }

    /// insert an element into the slot by initializing directly into the slot
    #[cfg(feature = "init")]
    pub fn init<Init: init::Initializer<T, Error = core::convert::Infallible>>(self, init: Init) {
        let Ok(()) = self.try_init(init);
    }
}

impl<T> DenseSlotMap<T> {
    /// Create a new [`DenseSlotMap`]
    pub const fn new() -> Self {
        Self {
            arena: GenericDenseArena::new(),
        }
    }

    /// Get the number of elements in the [`DenseSlotMap`]
    pub const fn len(&self) -> usize {
        self.arena.tracker().len()
    }

    /// Returns true if there are no elements in the [`DenseSlotMap`]
    pub const fn is_empty(&self) -> bool {
        self.arena.tracker().is_empty()
    }

    /// Insert a new value into a [`DenseSlotMap`]
    pub fn insert(&mut self, value: T) -> ArenaKey {
        self.arena.insert(value)
    }

    /// Insert a new value that depends on the key into a [`DenseSlotMap`]
    pub fn insert_with(&mut self, value: impl FnOnce(ArenaKey) -> T) -> ArenaKey {
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
    /// i.e. [`DenseSlotMap::get`] would have returned [`Some`]
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
    /// i.e. [`DenseSlotMap::get`] would have returned [`Some`]
    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        // SAFETY: the caller ensures that this is correct
        unsafe { self.arena.get_unchecked_mut(key) }
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    pub fn try_remove(&mut self, key: ArenaKey) -> Option<T> {
        self.arena.try_remove(key)
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    pub fn remove(&mut self, key: ArenaKey) -> T {
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

    /// An unordered list of values in the [`DenseSlotMap`]
    pub const fn values(&self) -> &[T] {
        self.arena.values()
    }

    /// An mutable unordered list of values in the [`DenseSlotMap`]
    pub const fn values_mut(&mut self) -> &mut [T] {
        self.arena.values_mut()
    }

    /// An iterator over all the keys in the [`DenseSlotMap`]
    pub fn keys(&self) -> Keys<'_> {
        Keys {
            keys: self.arena.tracker().keys(),
        }
    }

    /// The mutable slice of values in this [`DenseSlotMap`]
    /// and the [`Keys`] of this [`DenseSlotMap`]
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

impl<T> Default for DenseSlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::ops::Index<ArenaKey> for DenseSlotMap<T> {
    type Output = T;

    fn index(&self, index: ArenaKey) -> &Self::Output {
        &self.arena[index]
    }
}

impl<T> core::ops::IndexMut<ArenaKey> for DenseSlotMap<T> {
    fn index_mut(&mut self, index: ArenaKey) -> &mut Self::Output {
        &mut self.arena[index]
    }
}

/// An iterator over the keys in a [`DenseSlotMap`]
pub struct Keys<'a> {
    keys: crate::dense_tracker::Keys<'a, ArenaKey, (), gw32, u32>,
}

impl ExactSizeIterator for Keys<'_> {}
impl Iterator for Keys<'_> {
    type Item = ArenaKey;

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
        let mut s = DenseSlotMap::new();
        let a = s.insert(10);
        let b = s.insert(20);
        assert_eq!(s[a], 10);
        assert_eq!(s[b], 20);
        s.remove(a);
        assert_eq!(s[b], 20);
    }
}
