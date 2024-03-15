use crate::{
    generation::NoGeneration,
    generic_dense::{self as dense, GenericDenseArena},
};

pub struct DenseSlab<T> {
    pub arena: GenericDenseArena<T, (), NoGeneration, usize>,
}

pub struct VacantSlot<'a, T> {
    pub slot: dense::VacantSlot<'a, T, (), NoGeneration, usize>,
}

impl<T> VacantSlot<'_, T> {
    pub fn key(&self) -> usize {
        self.slot.key()
    }

    pub fn insert(self, value: T) {
        self.slot.insert(value);
    }
}

impl<T> DenseSlab<T> {
    pub const fn new() -> Self {
        Self {
            arena: GenericDenseArena::new(()),
        }
    }

    pub fn len(&self) -> usize {
        self.arena.tracker().len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena.tracker().is_empty()
    }

    pub fn insert(&mut self, value: T) -> usize {
        self.arena.insert(value)
    }

    pub fn insert_with(&mut self, value: impl FnOnce(usize) -> T) -> usize {
        self.arena.insert_with(value)
    }

    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T> {
        VacantSlot {
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
        self.arena.try_remove(key)
    }

    pub fn remove(&mut self, key: usize) -> T {
        self.arena.remove(key)
    }

    pub unsafe fn remove_unchecked(&mut self, key: usize) -> T {
        unsafe { self.arena.remove_unchecked(key) }
    }

    pub fn values(&self) -> &[T] {
        self.arena.values()
    }

    pub fn values_mut(&mut self) -> &mut [T] {
        self.arena.values_mut()
    }

    pub fn keys(&self) -> Keys<'_> {
        Keys {
            keys: self.arena.tracker().keys(),
        }
    }

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
