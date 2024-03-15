use core::{mem::MaybeUninit, ops, ptr::NonNull};

use alloc::vec::Vec;

use crate::{
    generation::{DefaultGeneration, Generation},
    generic_sparse::{self as sparse, GenericSparseArena},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

pub struct GenericDenseTracker<O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    index_rev: Vec<MaybeUninit<I>>,
    index_fwd: GenericSparseArena<I, O, G, I>,
}

pub struct VacantSlot<'a, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    sparse: sparse::VacantSlot<'a, I, O, G, I>,
}

struct Aliased<T>(NonNull<T>);
unsafe impl<T: Send> Send for Aliased<T> {}
unsafe impl<T: Sync> Sync for Aliased<T> {}

impl<O, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I> {
    pub const unsafe fn new(owner: O) -> Self {
        Self {
            index_rev: Vec::new(),
            index_fwd: GenericSparseArena::new(owner),
        }
    }
}

impl<O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, O, G, I> {
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        self.sparse.key()
    }

    pub fn insert(self, len: usize) {
        self.sparse.insert(I::from_usize(len))
    }
}

impl<O, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I> {
    pub fn vacant_slot(&mut self, len: usize) -> VacantSlot<'_, O, G, I> {
        let slot = self.index_fwd.vacant_slot();
        if self.index_rev.len() == len {
            self.index_rev.reserve(1);
            // MaybeUninit is always initialized, even for uninitialized bytes
            unsafe { self.index_rev.set_len(self.index_rev.capacity()) }
        }
        let index_rev = unsafe { self.index_rev.get_unchecked_mut(len) };
        *index_rev = MaybeUninit::new(I::from_usize(slot.key::<usize>()));

        VacantSlot {
            sparse: self.index_fwd.vacant_slot(),
        }
    }

    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<usize> {
        self.index_fwd.get(key).copied().map(I::to_usize)
    }

    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> usize {
        unsafe { *self.index_fwd.get_unchecked(key) }.to_usize()
    }

    fn remove_at(&mut self, len: usize, index_fwd: I) -> usize {
        let end = len.wrapping_sub(1);
        // we are going to swap remove index_fwd out, so we will need to update the mappings
        // of the end of the list
        let index_end_rev = unsafe { self.index_rev.get_unchecked(end).assume_init_read() };

        // we need to update the forward mapping to show that the end is now pointing to index_fwd
        unsafe { *self.index_fwd.get_unchecked_mut(index_end_rev.to_usize()) = index_fwd };
        let index_fwd = index_fwd.to_usize();

        // the end is now at index_fwd so we need to update the reverse mapping accordingly
        unsafe { *self.index_rev.get_unchecked_mut(index_fwd) = MaybeUninit::new(index_end_rev) }

        // this is to eliminate the bounds check in self.values.swap_remove
        if index_fwd.to_usize() >= len {
            unsafe { core::hint::unreachable_unchecked() }
        }

        index_fwd
    }

    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, len: usize, key: K) -> Option<usize> {
        let index_fwd = self.index_fwd.try_remove(key)?;
        Some(self.remove_at(len, index_fwd))
    }

    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, len: usize, key: K) -> usize {
        let index_fwd = self.index_fwd.remove(key);
        self.remove_at(len, index_fwd)
    }

    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, len: usize, key: K) -> usize {
        let index_fwd = unsafe { self.index_fwd.remove_unchecked(key) };
        self.remove_at(len, index_fwd)
    }
}

impl<K: ArenaIndex<O, G>, O: ?Sized, G: Generation, I: InternalIndex> ops::Index<K>
    for GenericDenseTracker<O, G, I>
{
    type Output = I;

    fn index(&self, index: K) -> &Self::Output {
        &self.index_fwd[index]
    }
}
