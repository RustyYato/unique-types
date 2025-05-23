//! An implementation of sparse arenas with a lot of knobs to tweak
//!
//! see [`GenericSparseArena`] for details

use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops,
};

use ut_vec::{UtVec, UtVecElementIndex};

use crate::{
    generation::{DefaultGeneration, Generation},
    internal_index::InternalIndex,
    key::ArenaIndex,
};

/// A [`GenericSparseArena`] is a small wrapper around a `Vec<(Generation, T)>`
///
/// see the crate level docs for usage and considerations, the rest of the docs here
/// will go over implementation details as exposition
///
/// ## Implementation details
///
/// It stores elements like so:
///
/// ```text
/// free_list_head: 1
/// data: [ [ generation1, value1 ], [ generation2, 3 ], [ generation3, value2 ], [ generation4, 4 ], ]
/// ```
///
/// Each element of the list is a `Slot`, each slot can be in one of two states:
/// * Empty: then it stores the generation and the next empty slot. If there are no other empty
///   slots, then it holds the an index to one past the end of the list.
///   For example, above slot4 points to 4, which is one past the end of the list.
/// * Filled: then it stores the generation and the value it's filled with.
///
/// The generation is responsible for tracking if a slot is empty or filled, so we don't need any
/// other way to discriminate between them.
///
/// the free_list_head points to the first element of the free list, or one past the end of the
/// list if there are no more elements.
///
/// On insertion,
/// 1. if the free_list_head points to one past the end, push a new slot
/// 2. now the free_list_head points to a valid slot
/// 3. insert the value into the slot pointed to by the free_list_head
/// 4. increment the generation of the slot
///
/// On access,
/// 1. Check that the key is in bounds
/// 2. check the generation of the indexed slot, and return an error if they fail
///   * if the key is [`usize`], or [`UtIndex`](ut_vec::UtIndex) then check if the generation
///     represents a filled generation
///   * if the key is [`ArenaKey`](crate::key::ArenaKey), then check if the key's generation
///     matches the slot's generation
/// 3. return the slot's value
///
/// On removal,
/// 1. do "On access,"
/// 2. remove the value from the slot
/// 3. try to increment the generation
///   * on success, write free_list_head to the slot, then set free_list_head to the index of
///     the slot
///   * on failure write [`Generation::EMPTY`] as the generation and don't modify free_list_head
///     (thus "leaking" the slot, as it can no longer be used at all).
/// 4. return the value
///
/// All of these operations are constant time, with low overhead.
#[derive(Debug)]
pub struct GenericSparseArena<
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    // this can be usize, since any smaller type won't make GenericArena any smaller
    // because we will round up to padding
    free_list_head: usize,
    slots: ut_vec::UtVec<Slot<T, G, I>, O>,
}

impl<T: core::fmt::Debug, G: Generation, I: InternalIndex> core::fmt::Debug for Slot<T, G, I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: accessing `Slot` is safe if the generation says it is filled
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
struct EmptySlot<G: Generation, I: InternalIndex> {
    generation: G,
    next_empty_slot: I,
}

#[repr(C)]
#[derive(Debug)]
struct FilledSlot<T, G: Generation> {
    generation: G,
    value: T,
}

#[repr(C)]
union Slot<T, G: Generation, I: InternalIndex> {
    generation: G,
    filled: ManuallyDrop<FilledSlot<T, G>>,
    empty: EmptySlot<G, I>,
}

impl<T, G: Generation, I: InternalIndex> Drop for Slot<T, G, I> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() && self.generation().is_filled() {
            // SAFETY: the generation says this slot is filled
            // and no one else can access elements after they have been dropped
            unsafe { ManuallyDrop::drop(&mut self.filled) }
        }
    }
}

/// a vacant slot into the [`GenericSparseArena`], created via [`GenericSparseArena::vacant_slot`]
pub struct VacantSlot<
    'a,
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    free_list_head: &'a mut usize,
    slot: &'a mut Slot<T, G, I>,
    owner: &'a O,
    next_empty_slot: usize,
}

impl<T, G: Generation, I: InternalIndex> Slot<T, G, I> {
    const fn generation(&self) -> G {
        // SAFETY: all variants of the union have the generation at the start
        unsafe { self.generation }
    }

    unsafe fn remove(&mut self, index: usize, free_list_head: &mut usize) -> T {
        let generation = self.generation();

        // try to insert the slot into the free-list if the generation is not yet exhausted
        let (next_empty_slot, generation) =
            // SAFETY: the caller ensures that this slot is full, so calling try_empty is safe
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
                    // SAFETY: the caller ensures that the index is in bounds, and free_list_head
                    // are in bounds
                    next_empty_slot: unsafe { I::from_usize_unchecked(next_empty_slot) },
                },
            },
        );

        let slot = ManuallyDrop::new(slot);
        // SAFETY: the caller ensures that this slot is filled
        // and we don't drop slot, so value isn't double dropped
        unsafe { core::ptr::read(&slot.filled.value) }
    }
}

impl<T, O: ?Sized, G: Generation, I: InternalIndex> VacantSlot<'_, T, O, G, I> {
    /// Get the key that will be associated with this slot once it is filled
    pub fn key<K: ArenaIndex<O, G>>(&self) -> K {
        // SAFETY: the slot is guaranteed to be empty, so we can just fill it and then
        // it is guaranteed to be filled, so we can call to_filled
        let generation = unsafe { self.slot.generation.fill().to_filled() };
        // SAFETY: self.last_empty is guaranteed to be in bounds of arena.slots (it's the index of
        // self.slot)
        unsafe { K::new(*self.free_list_head, self.owner, generation) }
    }

    /// Insert an element into this slot
    #[inline]
    pub fn insert(self, value: T) {
        // SAFETY: [`GenericSparseArena::vacant_slot`] ensures that this slot
        // is empty
        // and it's not possible to call [`Self::insert`] multiple times
        // casting FilledSlot<T, G> to FilledSlot<MaybeUninit<T>, G> is legal
        // because FilledSlot is repr(C), and MaybeUninit<T> has the same repr as T
        // and because FilledSlot just stores a T, and doesn't do anything fancy with it
        let slot = unsafe {
            &mut *(self.slot as *mut Slot<T, G, I> as *mut FilledSlot<MaybeUninit<T>, G>)
        };

        // NOTE: since the first thing we do is write to value, it is very likely
        // that the value will be directly written into slot.value when optimizations
        // are turned on.
        slot.value = MaybeUninit::new(value);

        // SAFETY: [`GenericSparseArena::vacant_slot`] ensures that the slot
        // is empty
        // and it's not possible to call [`Self::insert`] multiple times
        unsafe { slot.generation = slot.generation.fill() }

        // update the next_empty_slot to point to the slot after the next slot
        *self.free_list_head = self.next_empty_slot;
    }
}

impl<T, G: Generation, I: InternalIndex> GenericSparseArena<T, (), G, I> {
    /// Create a new [`GenericSparseArena`]
    pub const fn new() -> Self {
        Self {
            free_list_head: 0,
            slots: UtVec::new(),
        }
    }
}

impl<T, G: Generation, I: InternalIndex> Default for GenericSparseArena<T, (), G, I> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "unique-types")]
impl<T, O, G: Generation, I: InternalIndex> GenericSparseArena<T, O, G, I> {
    /// Create a new [`GenericSparseArena`] with the given owner
    pub const fn with_owner(owner: O) -> Self
    where
        O: unique_types::UniqueToken,
    {
        Self {
            free_list_head: 0,
            slots: UtVec::from_owner(owner),
        }
    }

    /// Get the owner of this type's keys
    pub const fn owner(&self) -> &O {
        self.slots.owner()
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

    /// Access a vacant slot in the arena
    #[inline]
    pub fn vacant_slot(&mut self) -> VacantSlot<'_, T, O, G, I> {
        if self.free_list_head == self.slots.len() {
            self.reserve_vacant_slot_slow();
        }

        let (slots, owner) = self.slots.as_mut_slice_and_owner();
        // SAFETY: reserve_vacant_slot_slow ensures that free_list_head points to a
        // valid element of slots
        let slot = unsafe { slots.get_unchecked_mut(self.free_list_head) };

        VacantSlot {
            // SAFETY: free_list_head always points to an empty slot
            next_empty_slot: unsafe { slot.empty }.next_empty_slot.to_usize(),
            slot,
            free_list_head: &mut self.free_list_head,
            owner,
        }
    }

    /// Insert a new value into a [`GenericSparseArena`]
    #[inline]
    pub fn insert<K: ArenaIndex<O, G>>(&mut self, value: T) -> K {
        if self.free_list_head == self.slots.len() {
            self.slots.push(Slot {
                filled: ManuallyDrop::new(FilledSlot {
                    // SAFETY: G::EMPTY is guaranteed to be empty, so we can fill it
                    generation: unsafe { G::EMPTY.fill() },
                    value,
                }),
            });

            let index = self.free_list_head;
            self.free_list_head += 1;

            // SAFETY: G::EMPTY is guaranteed to be empty, so we can fill it
            // and self.free_list_head is guaranteed to be a valid index
            unsafe { K::new(index, self.slots.owner(), G::EMPTY.fill().to_filled()) }
        } else {
            self.insert_with(move |_| value)
        }
    }

    /// Insert a new value that depends on the key into a [`GenericSparseArena`]
    #[inline]
    pub fn insert_with<K: ArenaIndex<O, G>>(&mut self, value: impl FnOnce(K) -> T) -> K {
        let slot = self.vacant_slot();
        let key = slot.key();
        slot.insert(value(key));
        key
    }

    /// Get a reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn get<K: ArenaIndex<O, G>>(&self, key: K) -> Option<&T> {
        let slot = self.slots.get(key.to_index())?;
        if key.matches_generation(slot.generation()) {
            debug_assert!(slot.generation().is_filled());
            // SAFETY: if the slot's generation matches the key's generation
            // then it must be filled. Since keys only hold filled generations
            Some(unsafe { &slot.filled.value })
        } else {
            None
        }
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// Returns None if the key is invalid (out of bounds, or incorrect generation)
    #[inline]
    pub fn get_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.to_index())?;
        if key.matches_generation(slot.generation()) {
            debug_assert!(slot.generation().is_filled());
            // SAFETY: if the slot's generation matches the key's generation
            // then it must be filled. Since keys only hold filled generations
            Some(unsafe { &mut slot.filled.value })
        } else {
            None
        }
    }

    /// Get a reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and must have the correct generation
    ///
    /// i.e. [`GenericSparseArena::get`] would have returned [`Some`]
    #[inline]
    pub unsafe fn get_unchecked<K: ArenaIndex<O, G>>(&self, key: K) -> &T {
        // SAFETY: the caller ensures that the key is in bounds
        let slot = unsafe { self.slots.get_unchecked(key.to_index()) };

        debug_assert!(slot.generation().is_filled());
        debug_assert!(key.matches_generation(slot.generation()));

        // SAFETY: The caller ensures that the slot's generation matches
        // the key's generation
        // if the slot's generation matches the key's generation
        // then it must be filled. Since keys only hold filled generations
        unsafe { &slot.filled.value }
    }

    /// Get a mutable reference to the value associated with the key
    ///
    /// # Safety
    ///
    /// The key must be in bounds and must have the correct generation
    ///
    /// i.e. [`GenericSparseArena::get_mut`] would have returned [`Some`]
    #[inline]
    pub unsafe fn get_unchecked_mut<K: ArenaIndex<O, G>>(&mut self, key: K) -> &mut T {
        // SAFETY: the caller ensures that the key is in bounds
        let slot = unsafe { self.slots.get_unchecked_mut(key.to_index()) };

        debug_assert!(slot.generation().is_filled());
        debug_assert!(key.matches_generation(slot.generation()));

        // SAFETY: The caller ensures that the slot's generation matches
        // the key's generation
        // if the slot's generation matches the key's generation
        // then it must be filled. Since keys only hold filled generations
        unsafe { &mut slot.filled.value }
    }

    /// Get the key associated with an index into the arena
    ///
    /// Returns [`None`] if the index points to an empty slot, or is out of bounds
    #[inline]
    pub fn try_key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> Option<K> {
        let slot = self.slots.get(index)?;
        if slot.generation().is_filled() {
            debug_assert!(slot.generation().is_filled());
            // SAFETY: self.get ensures that the index is in bounds
            // and we have checked that the generation is filled
            Some(unsafe { K::new(index, self.slots.owner(), slot.generation().to_filled()) })
        } else {
            None
        }
    }

    /// Get the key associated with an index into the arena
    ///
    /// # Panics
    ///
    /// If the index points to an empty slot, or is out of bounds
    #[inline]
    pub fn key_of<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        let slot = &self.slots[index];
        if slot.generation().is_empty() {
            crate::key::access_empty_slot(index)
        }
        // SAFETY: self.get ensures that the index is in bounds
        // and we have checked that the generation is filled
        unsafe { K::new(index, self.slots.owner(), slot.generation().to_filled()) }
    }

    /// Get the key associated with an index into the arena
    ///
    /// # Safety
    ///
    /// The index must be in bounds and must point to a filled slot
    #[inline]
    pub unsafe fn key_of_unchecked<K: ArenaIndex<O, G>>(&self, index: usize) -> K {
        // SAFETY: the caller ensures that the index is in bounds
        let slot = unsafe { self.slots.get_unchecked(index) };
        debug_assert!(slot.generation().is_filled());

        // SAFETY: the caller ensures that the generation is filled
        unsafe { K::new(index, self.slots.owner(), slot.generation().to_filled()) }
    }

    /// Try to remove the element associated with the key
    ///
    /// Returns None if the key is invalid or out of bounds
    #[inline]
    pub fn try_remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> Option<T> {
        let index = key.to_index();
        let slot = self.slots.get_mut(index)?;
        let index = index.get_index();
        if key.matches_generation(slot.generation()) {
            debug_assert!(slot.generation().is_filled());

            // SAFETY: self.get ensures that the index is in bounds
            // we have checked that the generation is filled
            // and free_list_head always points to a valid empty index
            Some(unsafe { slot.remove(index, &mut self.free_list_head) })
        } else {
            None
        }
    }

    /// Try to remove the element associated with the key
    ///
    /// # Panics
    ///
    /// if the key is invalid or out of bounds
    #[inline]
    pub fn remove<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = key.to_index();
        let slot = &mut self.slots[index];
        let index = index.get_index();
        key.assert_matches_generation(slot.generation());
        debug_assert!(slot.generation().is_filled());

        // SAFETY: self.get ensures that the index is in bounds
        // we have checked that the generation is filled
        // and free_list_head always points to a valid empty index
        unsafe { slot.remove(index, &mut self.free_list_head) }
    }

    /// Remove the element associated with the key without checking
    /// if the key is invalid or out of bounds
    ///
    /// # Safety
    ///
    /// They key must be in bounds, and point to a filled slot
    #[inline]
    pub unsafe fn remove_unchecked<K: ArenaIndex<O, G>>(&mut self, key: K) -> T {
        let index = key.to_index();
        // SAFETY: the caller ensures that the index is in bounds
        let slot = unsafe { self.slots.get_unchecked_mut(index) };
        debug_assert!(slot.generation().is_filled());
        let index = index.get_index();
        // SAFETY: the caller ensures that the slot is filled
        unsafe { slot.remove(index, &mut self.free_list_head) }
    }

    /// Get an iterator over the keys and references to elements of this arena
    #[inline]
    pub fn iter<K: ArenaIndex<O, G>>(&self) -> Iter<'_, K, T, O, G, I> {
        Iter {
            slots: self.slots.iter().enumerate(),
            owner: self.slots.owner(),
            _key: PhantomData,
        }
    }

    /// Get an iterator over the keys and mut references to elements of this arena
    #[inline]
    pub fn iter_mut<K: ArenaIndex<O, G>>(&mut self) -> IterMut<'_, K, T, O, G, I> {
        let (slots, owner) = self.slots.as_mut_slice_and_owner();
        IterMut {
            slots: slots.iter_mut().enumerate(),
            owner,
            _key: PhantomData,
        }
    }

    /// Get an iterator over the keys of this arena
    #[inline]
    pub fn keys<K: ArenaIndex<O, G>>(&self) -> Keys<'_, K, T, O, G, I> {
        Keys { iter: self.iter() }
    }

    /// Get an iterator over the references to elements of this arena
    #[inline]
    pub fn values(&self) -> Values<'_, T, G, I> {
        Values {
            slots: self.slots.iter(),
        }
    }

    /// Get an iterator over the mut references to elements of this arena
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, T, G, I> {
        ValuesMut {
            slots: self.slots.iter_mut(),
        }
    }
}

impl<K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> ops::Index<K>
    for GenericSparseArena<T, O, G, I>
{
    type Output = T;

    fn index(&self, index: K) -> &Self::Output {
        let slot = &self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        debug_assert!(slot.generation().is_filled());
        // SAFETY: The caller ensures that the slot's generation matches
        // the key's generation
        // if the slot's generation matches the key's generation
        // then it must be filled. Since keys only hold filled generations
        unsafe { &slot.filled.value }
    }
}

impl<K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> ops::IndexMut<K>
    for GenericSparseArena<T, O, G, I>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        let slot = &mut self.slots[index.to_index()];
        index.assert_matches_generation(slot.generation());
        debug_assert!(slot.generation().is_filled());
        // SAFETY: The caller ensures that the slot's generation matches
        // the key's generation
        // if the slot's generation matches the key's generation
        // then it must be filled. Since keys only hold filled generations
        unsafe { &mut slot.filled.value }
    }
}

/// An iterator over references of values in a [`GenericSparseArena`], created from
/// [`GenericSparseArena::values`]
pub struct Values<'a, T, G: Generation = DefaultGeneration, I: InternalIndex = usize> {
    slots: core::slice::Iter<'a, Slot<T, G, I>>,
}

/// An iterator over mut references of values in a [`GenericSparseArena`], created from
/// [`GenericSparseArena::values_mut`]
pub struct ValuesMut<'a, T, G: Generation = DefaultGeneration, I: InternalIndex = usize> {
    slots: core::slice::IterMut<'a, Slot<T, G, I>>,
}

/// An iterator over keys and references of values in a [`GenericSparseArena`], created from
/// [`GenericSparseArena::values`]
pub struct Iter<
    'a,
    K,
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    slots: core::iter::Enumerate<core::slice::Iter<'a, Slot<T, G, I>>>,
    owner: &'a O,
    _key: PhantomData<fn() -> K>,
}

/// An iterator over keys and mutable references of values in a [`GenericSparseArena`], created from
/// [`GenericSparseArena::values`]
pub struct IterMut<
    'a,
    K,
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    slots: core::iter::Enumerate<core::slice::IterMut<'a, Slot<T, G, I>>>,
    owner: &'a O,
    _key: PhantomData<fn() -> K>,
}

/// An iterator over keys in a [`GenericSparseArena`], created from
/// [`GenericSparseArena::values`]
pub struct Keys<
    'a,
    K,
    T,
    O: ?Sized = (),
    G: Generation = DefaultGeneration,
    I: InternalIndex = usize,
> {
    iter: Iter<'a, K, T, O, G, I>,
}

impl<T, G: Generation, I: InternalIndex> Clone for Values<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
        }
    }
}

impl<T, G: Generation, I: InternalIndex> Clone for Iter<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            owner: self.owner,
            _key: PhantomData,
        }
    }
}

impl<T, G: Generation, I: InternalIndex> Clone for Keys<'_, T, G, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T, G: Generation, I: InternalIndex> Iterator for Values<'a, T, G, I> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|slot| {
            if slot.generation().is_filled() {
                // SAFETY: the generation says the slot is filled
                Some(unsafe { &slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: InternalIndex> DoubleEndedIterator for Values<'a, T, G, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|slot| {
            if slot.generation().is_filled() {
                // SAFETY: the generation says the slot is filled
                Some(unsafe { &slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: InternalIndex> Iterator for ValuesMut<'a, T, G, I> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|slot| {
            if slot.generation().is_filled() {
                // SAFETY: the generation says the slot is filled
                Some(unsafe { &mut slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, T, G: Generation, I: InternalIndex> DoubleEndedIterator for ValuesMut<'a, T, G, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|slot| {
            if slot.generation().is_filled() {
                // SAFETY: the generation says the slot is filled
                Some(unsafe { &mut slot.filled.value })
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> Iterator
    for Iter<'a, K, T, O, G, I>
{
    type Item = (K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                // SAFETY: Enumerate always yields valid indices
                // and we have ensured that the slot's generation is filled
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                // SAFETY: the generation says the slot is filled
                Some((key, unsafe { &slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> DoubleEndedIterator
    for Iter<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                // SAFETY: Enumerate always yields valid indices
                // and we have ensured that the slot's generation is filled
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                // SAFETY: the generation says the slot is filled
                Some((key, unsafe { &slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> Iterator
    for IterMut<'a, K, T, O, G, I>
{
    type Item = (K, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.slots.find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                // SAFETY: Enumerate always yields valid indices
                // and we have ensured that the slot's generation is filled
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                // SAFETY: the generation says the slot is filled
                Some((key, unsafe { &mut slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> DoubleEndedIterator
    for IterMut<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slots.by_ref().rev().find_map(|(i, slot)| {
            if slot.generation().is_filled() {
                // SAFETY: Enumerate always yields valid indices
                // and we have ensured that the slot's generation is filled
                let key = unsafe { ArenaIndex::new(i, self.owner, slot.generation().to_filled()) };
                // SAFETY: the generation says the slot is filled
                Some((key, unsafe { &mut slot.filled.value }))
            } else {
                None
            }
        })
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> Iterator
    for Keys<'a, K, T, O, G, I>
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, _)| key)
    }
}

impl<'a, K: ArenaIndex<O, G>, T, O: ?Sized, G: Generation, I: InternalIndex> DoubleEndedIterator
    for Keys<'a, K, T, O, G, I>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|(key, _)| key)
    }
}
