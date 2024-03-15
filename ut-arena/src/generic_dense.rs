use core::{mem::MaybeUninit, ops, ptr::NonNull};

use alloc::vec::Vec;

use crate::{
    generation::{DefaultGeneration, Generation},
    generic_sparse::{self as sparse, GenericSparseArena},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

pub struct GenericDenseArena<T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize>
{
    values: Vec<T>,
    // index_rev: Vec<usize>,
    index_fwd: GenericSparseArena<I, O, G, I>,
}

pub struct VacantSlot<'a, T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    sparse: sparse::VacantSlot<'a, I, O, G, I>,
    value: &'a mut MaybeUninit<T>,
    vec: Aliased<Vec<T>>,
}

struct Aliased<T>(NonNull<T>);
unsafe impl<T: Send> Send for Aliased<T> {}
unsafe impl<T: Sync> Sync for Aliased<T> {}

impl<T, O, G: Generation, I: InternalIndex> GenericDenseArena<T, O, G, I> {
    pub const fn new(owner: O) -> Self {
        Self {
            values: Vec::new(),
            index_fwd: GenericSparseArena::new(owner),
        }
    }
}

impl<T, O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, T, O, G, I> {
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        self.sparse.key()
    }

    pub fn insert(mut self, value: T) {
        *self.value = MaybeUninit::new(value);
        let index = unsafe {
            let vec = self.vec.0.as_mut();
            let index = vec.len();
            vec.set_len(index + 1);
            index
        };
        self.sparse.insert(I::from_usize(index))
    }
}

impl<T, O, G: Generation, I: InternalIndex> GenericDenseArena<T, O, G, I> {
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.values.len() == self.values.capacity() {
            self.values.reserve(1);
        }
        let mut values = NonNull::from(&mut self.values);
        let value = unsafe { values.as_mut() }.spare_capacity_mut();
        // SAFETY: there is guaranteed to be some spare capacity since we reserved space above
        let value = unsafe { value.get_unchecked_mut(0) };
        VacantSlot {
            sparse: self.index_fwd.vacant_slot(),
            value,
            vec: Aliased(values),
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
        let index = *self.index_fwd.get(key)?;
        Some(unsafe { self.values.get_unchecked(index.to_usize()) })
    }

    #[inline]
    pub fn get_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<&mut T> {
        let index = *self.index_fwd.get(key)?;
        Some(unsafe { self.values.get_unchecked_mut(index.to_usize()) })
    }

    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> &T {
        let index = unsafe { *self.index_fwd.get_unchecked(key) };
        unsafe { self.values.get_unchecked(index.to_usize()) }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> &mut T {
        let index = unsafe { *self.index_fwd.get_unchecked(key) };
        unsafe { self.values.get_unchecked_mut(index.to_usize()) }
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::Index<K>
    for GenericDenseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let index = self.index_fwd[index].to_usize();
        unsafe { self.values.get_unchecked(index) }
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex, T> ops::IndexMut<K>
    for GenericDenseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let index = self.index_fwd[index].to_usize();
        unsafe { self.values.get_unchecked_mut(index) }
    }
}
