use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops,
};

use ut_vec::{UtVec, UtVecElementIndex};

use crate::{
    generation::{self, DefaultGeneration, Generation},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

#[derive(Debug)]
pub struct GenericSparseArena<T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize>
{
    // this can be usize, since any smaller type won't make GenericArena any smaller
    // because we will round up to padding
    free_list_head: usize,
    slots: ut_vec::UtVec<Slot<T, G, I>, O>,
}

impl<T: core::fmt::Debug, G: Generation + core::fmt::Debug, I: Copy + core::fmt::Debug>
    core::fmt::Debug for Slot<T, G, I>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe {
            if self.generation().is_filled() {
                (*self.filled).fmt(f)
            } else {
                self.empty.fmt(f)
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct EmptySlot<G: Copy, I: Copy> {
    generation: G,
    next_empty_slot: I,
}

#[repr(C)]
#[derive(Debug)]
struct FilledSlot<T, G: Copy> {
    generation: G,
    value: T,
}

#[repr(C)]
union Slot<T, G: Generation, I: Copy> {
    generation: G,
    filled: ManuallyDrop<FilledSlot<T, G>>,
    empty: EmptySlot<G, I>,
}

impl<T, G: Generation, I: Copy> Drop for Slot<T, G, I> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() && self.generation().is_filled() {
            unsafe { ManuallyDrop::drop(&mut self.filled) }
        }
    }
}

pub struct VacantSlot<'a, T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    next_empty_slot: &'a mut usize,
    slot: &'a mut Slot<T, G, I>,
    owner: &'a O,
    next_next_empty_slot: usize,
}

impl<T, G: Generation, I: Copy> Slot<T, G, I> {
    fn generation(&self) -> G {
        unsafe { self.generation }
    }
}

impl<T, G: Generation, I: InternalIndex> Slot<T, G, I> {
    unsafe fn remove(&mut self, index: usize, free_list_head: &mut usize) -> T {
        let generation = self.generation();

        // try to insert the slot into the free-list if the generation is not yet exhausted
        let (next_empty_slot, generation) =
            if let Ok(generation) = unsafe { generation.try_empty() } {
                let next_empty_slot = core::mem::replace(free_list_head, index);

                (next_empty_slot, generation)
            } else {
                (index, G::EMPTY)
            };

        let slot = core::mem::replace(
            self,
            Slot {
                empty: EmptySlot {
                    generation,
                    next_empty_slot: unsafe { I::from_usize_unchecked(next_empty_slot) },
                },
            },
        );

        let slot = ManuallyDrop::new(slot);
        unsafe { core::ptr::read(&slot.filled.value) }
    }
}

impl<T, O: ?Sized, G: Generation, I: Copy> VacantSlot<'_, T, O, G, I> {
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        // SAFETY: the slot is guaranteed to be empty, so we can just fill it and then
        // it is guaranteed to be filled, so we can call to_filled
        let genration = unsafe { self.slot.generation.fill().to_filled() };
        // SAFETY: self.last_empty is guaranteed to be in bounds of arena.slots (it's the index of
        // self.slot)
        unsafe { K::new(*self.next_empty_slot, self.owner, genration) }
    }

    pub fn insert(self, value: T) {
        let slot = unsafe {
            &mut *(self.slot as *mut Slot<T, G, I> as *mut FilledSlot<MaybeUninit<T>, G>)
        };

        slot.value = MaybeUninit::new(value);
        unsafe { slot.generation = slot.generation.fill() }
        *self.next_empty_slot = self.next_next_empty_slot;
    }
}

impl<T, O, G: Generation, I: InternalIndex> GenericSparseArena<T, O, G, I> {
    pub const fn new(owner: O) -> Self {
        Self {
            free_list_head: 0,
            slots: UtVec::new(owner),
        }
    }
}

impl<T, O: ?Sized, G: Generation, I: InternalIndex> GenericSparseArena<T, O, G, I> {
    #[cold]
    #[inline(never)]
    fn reserve_vacant_slot_slow(&mut self) {
        self.slots.push(Slot {
            empty: EmptySlot {
                generation: G::EMPTY,
                next_empty_slot: I::from_usize(self.free_list_head + 1),
            },
        });
    }

    #[inline]
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.free_list_head == self.slots.len() {
            self.reserve_vacant_slot_slow();
        }

        let (slots, owner) = self.slots.as_mut_slice_and_owner();
        let slot = unsafe { slots.get_unchecked_mut(self.free_list_head) };

        VacantSlot {
            next_next_empty_slot: unsafe { slot.empty }.next_empty_slot.to_usize(),
            slot,
            next_empty_slot: &mut self.free_list_head,
            owner,
        }
    }

    #[inline]
    pub fn insert<K: ArenaIndex<O, G>>(&mut self, value: T) -> K {
        self.insert_with(move |_| value)
    }

    #[inline]
    pub fn insert_with<K: ArenaIndex<O, G>>(&mut self, value: impl FnOnce(K) -> T) -> K {
        let slot = self.vacant_slot();
        let key = slot.key();
        slot.insert(value(key));
        key
    }

    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<&T> {
        let slot = self.slots.get(key.to_index())?;
        if key.matches_generation(slot.generation()) {
            Some(unsafe { &slot.filled.value })
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.to_index())?;
        if key.matches_generation(slot.generation()) {
            Some(unsafe { &mut slot.filled.value })
        } else {
            None
        }
    }

    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> &T {
        let slot = self.slots.get_unchecked(key.to_index());
        unsafe { &slot.filled.value }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> &mut T {
        let slot = unsafe { self.slots.get_unchecked_mut(key.to_index()) };
        unsafe { &mut slot.filled.value }
    }

    #[inline]
    pub fn try_key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        if slot.generation().is_filled() {
            Some(unsafe { K::new(index, &self.slots.owner(), slot.generation().to_filled()) })
        } else {
            None
        }
    }

    #[inline]
    pub fn key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        let slot = &self.slots[index];
        if slot.generation().is_empty() {
            crate::key::access_empty_slot(index)
        }
        unsafe { K::new(index, &self.slots.owner(), slot.generation().to_filled()) }
    }

    #[inline]
    pub unsafe fn key_of_unchecked<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        let slot = unsafe { self.slots.get_unchecked(index) };
        unsafe { K::new(index, &self.slots.owner(), slot.generation().to_filled()) }
    }

    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = key.to_index();
        let slot = &mut self.slots[index];
        let index = index.get_index();
        key.assert_matches_generation(slot.generation());
        unsafe { slot.remove(index, &mut self.free_list_head) }
    }

    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<T> {
        let index = key.to_index();
        let slot = self.slots.get_mut(index)?;
        let index = index.get_index();
        if key.matches_generation(slot.generation()) {
            Some(unsafe { slot.remove(index, &mut self.free_list_head) })
        } else {
            None
        }
    }

    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = key.to_index();
        let slot = self.slots.get_unchecked_mut(index);
        let index = index.get_index();
        unsafe { slot.remove(index, &mut self.free_list_head) }
    }

    #[inline]
    pub fn iter<K: ArenaIndex<O, G>>(&self) -> Iter<'_, K, T, O, G, I> {
        Iter {
            slots: self.slots.iter().enumerate(),
            owner: self.slots.owner(),
            _key: PhantomData,
        }
    }

    #[inline]
    pub fn iter_mut<K: ArenaIndex<O, G>>(&mut self) -> IterMut<'_, K, T, O, G, I> {
        let (slots, owner) = self.slots.as_mut_slice_and_owner();
        IterMut {
            slots: slots.iter_mut().enumerate(),
            owner,
            _key: PhantomData,
        }
    }

    #[inline]
    pub fn keys<K: ArenaIndex<O, G>>(&self) -> Keys<'_, K, T, O, G, I> {
        Keys { iter: self.iter() }
    }

    #[inline]
    pub fn values(&self) -> Values<'_, T, G, I> {
        Values {
            slots: self.slots.iter(),
        }
    }

    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, T, G, I> {
        ValuesMut {
            slots: self.slots.iter_mut(),
        }
    }
}

impl<K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: Copy> ops::Index<K>
    for GenericSparseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let slot = &self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        unsafe { &slot.filled.value }
    }
}

impl<K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: Copy> ops::IndexMut<K>
    for GenericSparseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let slot = &mut self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        unsafe { &mut slot.filled.value }
    }
}

pub struct Values<'a, T, G: Generation = DefaultGeneration, I: Copy = usize> {
    slots: core::slice::Iter<'a, Slot<T, G, I>>,
}

pub struct ValuesMut<'a, T, G: Generation = DefaultGeneration, I: Copy = usize> {
    slots: core::slice::IterMut<'a, Slot<T, G, I>>,
}

pub struct Iter<'a, K, T, O = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    slots: core::iter::Enumerate<core::slice::Iter<'a, Slot<T, G, I>>>,
    owner: &'a O,
    _key: PhantomData<fn() -> K>,
}

pub struct IterMut<'a, K, T, O = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    slots: core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, G, I>>>,
    owner: &'a O,
    _key: PhantomData<fn() -> K>,
}

pub struct Keys<'a, K, T, O = (), G: Generation = DefaultGeneration, I: Copy = usize> {
    iter: Iter<'a, K, T, O, G, I>,
}

impl<T, G: Generation, I: Copy> Clone for Values<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
        }
    }
}

impl<T, G: Generation, I: Copy> Clone for Iter<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            owner: self.owner,
            _key: PhantomData,
        }
    }
}

impl<T, G: Generation, I: Copy> Clone for Keys<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T, G: Generation, I: Copy> Iterator for Values<'a, T, G, I> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|slot| {
            if slot.generation().is_filled() {
                Some(unsafe { &slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: Copy> DoubleEndedIterator for Values<'a, T, G, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|slot| {
            if slot.generation().is_filled() {
                Some(unsafe { &slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: Copy> Iterator for ValuesMut<'a, T, G, I> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|slot| {
            if slot.generation().is_filled() {
                Some(unsafe { &mut slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: Copy> DoubleEndedIterator for ValuesMut<'a, T, G, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|slot| {
            if slot.generation().is_filled() {
                Some(unsafe { &mut slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> Iterator for Iter<'a, K, T, O, G, I> {
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                Some((key, unsafe { &slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> DoubleEndedIterator
    for Iter<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                Some((key, unsafe { &slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> Iterator
    for IterMut<'a, K, T, O, G, I>
{
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                Some((key, unsafe { &mut slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> DoubleEndedIterator
    for IterMut<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                Some((key, unsafe { &mut slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> Iterator for Keys<'a, K, T, O, G, I> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, _)| key)
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> DoubleEndedIterator
    for Keys<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(key, _)| key)
    }
}
