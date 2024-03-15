use std::mem::{ManuallyDrop, MaybeUninit};

use unique_types::UniqueToken;
use ut_vec::{UtIndex, UtVec, UtVecIndex};

use crate::{
    generation::{DefaultGeneration, Generation},
    index::InternalIndex,
};

pub struct GenericSparseArena<T, O: ?Sized = (), G: Copy = DefaultGeneration, I: Copy = usize> {
    // this can be usize, since any smaller type won't make GenericArena any smaller
    // because we will round up to padding
    last_empty: usize,
    slots: ut_vec::UtVec<Slot<T, G, I>, O>,
}

#[derive(Clone, Copy)]
pub struct ArenaKey<I, G: Generation = DefaultGeneration> {
    index: I,
    generation: G::Filled,
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
union Slot<T, G: Copy, I: Copy> {
    generation: G,
    filled: ManuallyDrop<FilledSlot<T, G>>,
    empty: EmptySlot<G, I>,
}

pub struct VacantSlot<'a, T, O: ?Sized = (), G: Copy = DefaultGeneration, I: Copy = usize> {
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

impl<T, O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, T, O, G, I> {
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
}

pub unsafe trait ArenaIndex<O: ?Sized, G: Generation>: Copy {
    type UtIndex: UtVecIndex<O, OutputKind = ut_vec::Element>;

    /// # Safety
    ///
    /// The index must be in bounds for the Arena that
    unsafe fn new(index: usize, owner: &O, generation: G::Filled) -> Self;

    fn to_index(&self) -> Self::UtIndex;

    fn matches_generation(self, g: G) -> bool;
}

unsafe impl<O: ?Sized, G: Generation> ArenaIndex<O, G> for usize {
    type UtIndex = Self;

    unsafe fn new(index: usize, _owner: &O, _generation: G::Filled) -> Self {
        index
    }

    fn to_index(&self) -> Self::UtIndex {
        *self
    }

    fn matches_generation(self, g: G) -> bool {
        g.is_filled()
    }
}

unsafe impl<O: ?Sized + UniqueToken, G: Generation> ArenaIndex<O, G> for UtIndex<O> {
    type UtIndex = Self;

    unsafe fn new(index: usize, owner: &O, _generation: G::Filled) -> Self {
        // the caller ensures that this is a valid index into the [`UtVec`] that owns owner
        unsafe { Self::new_unchecked(index, owner) }
    }

    fn to_index(&self) -> Self::UtIndex {
        *self
    }

    fn matches_generation(self, g: G) -> bool {
        g.is_filled()
    }
}

unsafe impl<O: ?Sized, G: Generation> ArenaIndex<O, G> for ArenaKey<usize, G> {
    type UtIndex = usize;

    unsafe fn new(index: usize, _owner: &O, generation: G::Filled) -> Self {
        Self { index, generation }
    }

    fn to_index(&self) -> Self::UtIndex {
        self.index
    }

    fn matches_generation(self, g: G) -> bool {
        g.matches(self.generation)
    }
}

unsafe impl<O: ?Sized + UniqueToken, G: Generation> ArenaIndex<O, G> for ArenaKey<UtIndex<O>, G> {
    type UtIndex = UtIndex<O>;

    unsafe fn new(index: usize, owner: &O, generation: G::Filled) -> Self {
        // SAFETY: the caller ensures that this is a valid index into the [`UtVec`] that owns owner
        Self {
            index: unsafe { UtIndex::new_unchecked(index, owner) },
            generation,
        }
    }

    fn to_index(&self) -> Self::UtIndex {
        self.index
    }

    fn matches_generation(self, g: G) -> bool {
        g.matches(self.generation)
    }
}
