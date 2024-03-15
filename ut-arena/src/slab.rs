use crate::{
    generation::NoGeneration,
    generic_sparse::{self as sparse, GenericSparseArena},
};

pub struct Slab<T> {
    len: usize,
    arena: GenericSparseArena<T, (), NoGeneration, usize>,
}

pub struct VacantSlot<'a, T> {
    len: &'a mut usize,
    slot: sparse::VacantSlot<'a, T, (), NoGeneration, usize>,
}

impl<T> VacantSlot<'_, T> {
    pub fn key(&self) -> usize {
        self.slot.key()
    }

    pub fn insert(self, value: T) {
        self.slot.insert(value);
        *self.len += 1;
    }
}

impl<T> Slab<T> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            arena: GenericSparseArena::new(()),
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, value: T) -> usize {
        self.len += 1;
        self.arena.insert(value)
    }

    pub fn insert_with(&mut self, value: impl FnOnce(usize) -> T) -> usize {
        self.len += 1;
        self.arena.insert_with(value)
    }

    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T> {
        VacantSlot {
            len: &mut self.len,
            slot: self.arena.vacant_slot(),
        }
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        self.arena.get(key)
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        self.arena.get_mut(key)
    }

    pub unsafe fn get_unchecked(&self, key: usize) -> &T {
        unsafe { self.arena.get_unchecked(key) }
    }

    pub unsafe fn get_unchecked_mut(&mut self, key: usize) -> &mut T {
        self.arena.get_unchecked_mut(key)
    }

    pub fn try_remove(&mut self, key: usize) -> Option<T> {
        let value = self.arena.try_remove(key);
        self.len -= value.is_some() as usize;
        value
    }

    pub fn remove(&mut self, key: usize) -> T {
        let value = self.arena.remove(key);
        self.len -= 1;
        value
    }

    pub unsafe fn remove_unchecked(&mut self, key: usize) -> T {
        self.len -= 1;
        unsafe { self.arena.remove_unchecked(key) }
    }

    pub fn values(&self) -> sparse::Values<'_, T, NoGeneration, usize> {
        self.arena.values()
    }

    pub fn values_mut(&mut self) -> sparse::ValuesMut<'_, T, NoGeneration, usize> {
        self.arena.values_mut()
    }

    pub fn keys(&self) -> sparse::Keys<'_, usize, T, (), NoGeneration, usize> {
        self.arena.keys()
    }

    pub fn iter(&self) -> sparse::Iter<'_, usize, T, (), NoGeneration, usize> {
        self.arena.iter()
    }

    pub fn iter_mut(&mut self) -> sparse::IterMut<'_, usize, T, (), NoGeneration, usize> {
        self.arena.iter_mut()
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
