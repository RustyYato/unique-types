use core::{
    mem::{ManuallyDrop, MaybeUninit},
    ops,
};

use ut_vec::UtVec;

use crate::{
    generation::{DefaultGeneration, Generation},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

pub struct GenericSparseArena<T, O: ?Sized = (), G: Generation = DefaultGeneration, I: Copy = usize>
{
    // this can be usize, since any smaller type won't make GenericArena any smaller
    // because we will round up to padding
    last_empty: usize,
    slots: ut_vec::UtVec<Slot<T, G, I>, O>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EmptySlot<G: Copy, I: Copy> {
    generation: G,
    next_empty_slot: I,
}

#[repr(C)]
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
    last_empty: &'a mut usize,
    slot: &'a mut Slot<T, G, I>,
    owner: &'a O,
    next_empty: usize,
}

impl<T, G: Generation, I: Copy> Slot<T, G, I> {
    pub fn generation(&self) -> G {
        unsafe { self.generation }
    }
}

impl<T, O: ?Sized, G: Generation, I: Copy> VacantSlot<'_, T, O, G, I> {
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        // SAFETY: the slot is guaranteed to be empty, so we can just fill it and then
        // it is guaranteed to be filled, so we can call to_filled
        let genration = unsafe { self.slot.generation.fill().to_filled() };
        // SAFETY: self.last_empty is guaranteed to be in bounds of arena.slots (it's the index of
        // self.slot)
        unsafe { K::new(*self.last_empty, self.owner, genration) }
    }

    pub fn insert(self, value: T) {
        let slot = unsafe {
            &mut *(self.slot as *mut Slot<T, G, I> as *mut FilledSlot<MaybeUninit<T>, G>)
        };

        slot.value = MaybeUninit::new(value);
        unsafe { slot.generation = slot.generation.fill() }
        *self.last_empty = self.next_empty;
    }
}

impl<T, O, G: Generation, I: InternalIndex> GenericSparseArena<T, O, G, I> {
    pub const fn new(owner: O) -> Self {
        Self {
            last_empty: 0,
            slots: UtVec::new(owner),
        }
    }

    #[cold]
    #[inline(never)]
    fn reserve_vacant_slot_slow(&mut self) {
        self.slots.push(Slot {
            empty: EmptySlot {
                generation: G::EMPTY,
                next_empty_slot: I::from_usize(self.last_empty + 1),
            },
        });
    }

    #[inline]
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.last_empty == self.slots.len() {
            self.reserve_vacant_slot_slow();
        }

        let (slots, owner) = self.slots.as_mut_slice_and_owner();
        let slot = unsafe { slots.get_unchecked_mut(self.last_empty) };

        VacantSlot {
            next_empty: unsafe { slot.empty }.next_empty_slot.to_usize(),
            slot,
            last_empty: &mut self.last_empty,
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
