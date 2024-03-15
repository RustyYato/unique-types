use core::{
    mem::{ManuallyDrop, MaybeUninit},
    ops,
};

use unique_types::UniqueToken;
use ut_vec::{UtIndex, UtVec, UtVecElementIndex};

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
}

#[cold]
#[inline(never)]
fn matches_generation_failed<G: Generation>(generation: G, filled: G::Filled, index: usize) -> ! {
    struct GenerationMatchFailed<G: Generation> {
        generation: G,
        filled: G::Filled,
        index: usize,
    }

    impl<G: Generation> core::fmt::Display for GenerationMatchFailed<G> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.generation.write_mismatch(self.filled, self.index, f)
        }
    }

    panic!(
        "{}",
        GenerationMatchFailed {
            generation,
            filled,
            index
        }
    )
}

#[cold]
#[inline(never)]
fn access_empty_slot(index: usize) -> ! {
    panic!("Tried to access empy slot at index: {index}")
}

impl<K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> ops::Index<K>
    for GenericSparseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let slot = &self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        unsafe { &slot.filled.value }
    }
}

impl<K: ArenaIndex<O, G>, T, O, G: Generation, I: Copy> ops::IndexMut<K>
    for GenericSparseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let slot = &mut self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        unsafe { &mut slot.filled.value }
    }
}

pub unsafe trait ArenaIndex<O: ?Sized, G: Generation>: Copy {
    type UtIndex: UtVecElementIndex<O>;

    /// # Safety
    ///
    /// The index must be in bounds for the Arena that
    unsafe fn new(index: usize, owner: &O, generation: G::Filled) -> Self;

    fn to_index(&self) -> Self::UtIndex;

    fn matches_generation(self, g: G) -> bool;

    fn assert_matches_generation(self, g: G);
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

    fn assert_matches_generation(self, g: G) {
        if g.is_empty() {
            access_empty_slot(self)
        }
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

    fn assert_matches_generation(self, g: G) {
        if g.is_empty() {
            access_empty_slot(self.get())
        }
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

    fn assert_matches_generation(self, g: G) {
        if !g.matches(self.generation) {
            matches_generation_failed(g, self.generation, self.index)
        }
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

    fn assert_matches_generation(self, g: G) {
        if !g.matches(self.generation) {
            matches_generation_failed(g, self.generation, self.index.get())
        }
    }
}
