use core::ops;

use alloc::vec::Vec;

use crate::{
    dense_tracker::{self, GenericDenseTracker},
    generation::{DefaultGeneration, Generation},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

pub struct GenericDenseArena<T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize>
{
    values: Vec<T>,
    tracker: GenericDenseTracker<O, G, I>,
}

pub struct VacantSlot<'a, T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    slot: dense_tracker::VacantSlot<'a, O, G, I>,
    vec: &'a mut Vec<T>,
}

impl<T, O, G: Generation, I: InternalIndex> GenericDenseArena<T, O, G, I> {
    pub const fn new(owner: O) -> Self {
        Self {
            values: Vec::new(),
            tracker: GenericDenseTracker::new(owner),
        }
    }
}

impl<T, O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, T, O, G, I> {
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        self.slot.key()
    }

    pub fn insert(self, value: T) {
        let index = self.slot.position();

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
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.values.len() == self.values.capacity() {
            self.values.reserve(1);
        }

        VacantSlot {
            slot: self.tracker.vacant_slot(self.values.len()),
            vec: &mut self.values,
        }
    }

    pub fn insert<K: ArenaIndex<O, G>>(&mut self, value: T) -> K {
        self.insert_with(move |_| value)
    }

    pub fn insert_with<K: ArenaIndex<O, G>>(&mut self, value: impl FnOnce(K) -> T) -> K {
        let slot = self.vacant_slot();
        let key = slot.key();
        slot.insert(value(key));
        key
    }

    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<&T> {
        let index = self.tracker.get(key)?;
        Some(unsafe { self.values.get_unchecked(index) })
    }

    #[inline]
    pub fn get_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<&mut T> {
        let index = self.tracker.get(key)?;
        Some(unsafe { self.values.get_unchecked_mut(index) })
    }

    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> &T {
        let index = unsafe { self.tracker.get_unchecked(key) };
        unsafe { self.values.get_unchecked(index) }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> &mut T {
        let index = unsafe { self.tracker.get_unchecked(key) };
        unsafe { self.values.get_unchecked_mut(index) }
    }

    fn remove_at(&mut self, index: usize) -> T {
        if index >= self.values.len() {
            unsafe { core::hint::unreachable_unchecked() }
        }

        self.values.swap_remove(index)
    }

    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<T> {
        let index = self.tracker.try_remove(key)?;
        Some(self.remove_at(index))
    }

    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = self.tracker.remove(key);
        self.remove_at(index)
    }

    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = self.tracker.remove_unchecked(key);
        self.remove_at(index)
    }

    #[inline]
    pub fn values(&self) -> &[T] {
        &self.values
    }

    #[inline]
    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::Index<K>
    for GenericDenseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let index = self.tracker[index].to_usize();
        unsafe { self.values.get_unchecked(index) }
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::IndexMut<K>
    for GenericDenseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let index = self.tracker[index].to_usize();
        unsafe { self.values.get_unchecked_mut(index) }
    }
}
