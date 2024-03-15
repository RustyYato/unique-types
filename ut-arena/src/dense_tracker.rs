use core::ops;

use alloc::vec::Vec;

use crate::{
    generation::{DefaultGeneration, Generation},
    generic_sparse::{self as sparse, GenericSparseArena},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

pub struct GenericDenseTracker<O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    index_rev: Vec<I>,
    index_fwd: GenericSparseArena<I, O, G, I>,
}

pub struct VacantSlot<'a, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    sparse: sparse::VacantSlot<'a, I, O, G, I>,
    index_rev: &'a mut Vec<I>,
}

impl<O, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I> {
    pub const fn new(owner: O) -> Self {
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

    pub fn position(&self) -> usize {
        self.index_rev.len()
    }

    pub fn insert(self) {
        let len = self.position();
        self.index_rev.push(I::from_usize(self.sparse.key()));
        self.sparse.insert(I::from_usize(len))
    }
}

impl<O, G: Generation, I: InternalIndex> GenericDenseTracker<O, G, I>
where
    O: core::fmt::Debug,
{
    pub fn vacant_slot(&mut self, len: usize) -> VacantSlot<'_, O, G, I> {
        assert_eq!(self.index_rev.len(), len);
        VacantSlot {
            sparse: self.index_fwd.vacant_slot(),
            index_rev: &mut self.index_rev,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.index_rev.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index_rev.is_empty()
    }

    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<usize> {
        self.index_fwd.get(key).copied().map(I::to_usize)
    }

    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> usize {
        unsafe { *self.index_fwd.get_unchecked(key) }.to_usize()
    }

    fn remove_at(&mut self, index_fwd: I) -> usize {
        if self.index_rev.is_empty() {
            unsafe { core::hint::unreachable_unchecked() }
        }
        if index_fwd.to_usize() >= self.index_rev.len() - 1 {
            unsafe { core::hint::unreachable_unchecked() }
        }

        self.index_rev.swap_remove(index_fwd.to_usize());
        let index_end_rev = self.index_rev[index_fwd.to_usize()];

        // we need to update the forward mapping to show that the end is now pointing to index_fwd
        unsafe { *self.index_fwd.get_unchecked_mut(index_end_rev.to_usize()) = index_fwd };

        index_fwd.to_usize()
    }

    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<usize> {
        let index_fwd = self.index_fwd.try_remove(key)?;
        Some(self.remove_at(index_fwd))
    }

    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> usize {
        let index_fwd = self.index_fwd.remove(key);
        self.remove_at(index_fwd)
    }

    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> usize {
        let index_fwd = unsafe { self.index_fwd.remove_unchecked(key) };
        self.remove_at(index_fwd)
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
