//! [`ArenaKey`] are the preferred keys of arenas which are specified through the [`ArenaIndex`] trait
//!
//! see them for more details

use ut_vec::UtVecElementIndex;

#[cfg(feature = "unique-types")]
use unique_types::UniqueToken;
#[cfg(feature = "unique-types")]
use ut_vec::UtIndex;

use crate::generation::{DefaultGeneration, Generation};

/// [`ArenaKey`] is just an index and a generation pair
///
/// The generation is a snapshot of the generation of the slot's genration
/// If the slot is removed, then this key will become invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArenaKey<I = usize, G: Generation = DefaultGeneration, Align = u64> {
    index: I,
    generation: G::Filled,
    _align: [Align; 0],
}

impl<I: core::hash::Hash, G: Generation, _Align> core::hash::Hash for ArenaKey<I, G, _Align> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        crate::key_hash::hash(&self.index, self.generation, state)
    }
}

impl<I, G: Generation> ArenaKey<I, G> {
    /// Get the underlying index type of [`ArenaKey`]
    #[inline]
    pub fn index(self) -> I {
        self.index
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
pub(crate) fn access_empty_slot(index: usize) -> ! {
    panic!("Tried to access empy slot at index: {index}")
}

/// A trait that manages access to arenas
///
/// # Safety
///
/// * `to_index` must not change what index it returns
/// * `matches_generation` should only succeed if the generation is filled
/// * `assert_matches_generation` should only return normally if `matches_generation` would have
///     returned true
pub unsafe trait ArenaIndex<O: ?Sized, G: Generation>: Copy {
    /// The underlying index type
    type UtIndex: UtVecElementIndex<O> + Copy;

    /// # Safety
    ///
    /// The index must be in bounds for the Arena that
    unsafe fn new(index: usize, owner: &O, generation: G::Filled) -> Self;

    /// Get the underlying index type
    fn to_index(&self) -> Self::UtIndex;

    /// Check that this key matches the generation, return false if it doesn't
    fn matches_generation(self, g: G) -> bool;

    /// Check that this key matches the generation, and panic if it doesn't
    fn assert_matches_generation(self, g: G);
}

// SAFETY: to_index always return self and *matches_generation only succeed if the generation is
// filled
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

#[cfg(feature = "unique-types")]
// SAFETY: to_index always return self and *matches_generation only succeed if the generation is
// filled
unsafe impl<O: ?Sized + UniqueToken, G: Generation> ArenaIndex<O, G> for UtIndex<O> {
    type UtIndex = Self;

    unsafe fn new(index: usize, owner: &O, _generation: G::Filled) -> Self {
        // SAFETY: the caller ensures that this is a valid index into the [`UtVec`] that owns owner
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

// SAFETY: to_index always return self.index and *matches_generation only succeed if the generation matches the key's
// filled generation. This is only possible if the generation is filled
unsafe impl<O: ?Sized, G: Generation, _Align: Copy> ArenaIndex<O, G> for ArenaKey<u32, G, _Align> {
    type UtIndex = usize;

    unsafe fn new(index: usize, _owner: &O, generation: G::Filled) -> Self {
        Self {
            index: index
                .try_into()
                .expect("Tried to push too many elements into Arena"),
            generation,
            _align: [],
        }
    }

    fn to_index(&self) -> Self::UtIndex {
        self.index as usize
    }

    fn matches_generation(self, g: G) -> bool {
        g.matches(self.generation)
    }

    fn assert_matches_generation(self, g: G) {
        if !g.matches(self.generation) {
            matches_generation_failed(g, self.generation, self.index as usize)
        }
    }
}

// SAFETY: to_index always return self.index and *matches_generation only succeed if the generation matches the key's
// filled generation. This is only possible if the generation is filled
unsafe impl<O: ?Sized, G: Generation, _Align: Copy> ArenaIndex<O, G>
    for ArenaKey<usize, G, _Align>
{
    type UtIndex = usize;

    unsafe fn new(index: usize, _owner: &O, generation: G::Filled) -> Self {
        Self {
            index,
            generation,
            _align: [],
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
            matches_generation_failed(g, self.generation, self.index)
        }
    }
}

#[cfg(feature = "unique-types")]
// SAFETY: to_index always return self.index and *matches_generation only succeed if the generation matches the key's
// filled generation. This is only possible if the generation is filled
unsafe impl<O: ?Sized + UniqueToken, G: Generation, _Align: Copy> ArenaIndex<O, G>
    for ArenaKey<UtIndex<O>, G, _Align>
{
    type UtIndex = UtIndex<O>;

    unsafe fn new(index: usize, owner: &O, generation: G::Filled) -> Self {
        Self {
            // SAFETY: the caller ensures that this is a valid index into the [`UtVec`] that owns owner
            index: unsafe { UtIndex::new_unchecked(index, owner) },
            generation,
            _align: [],
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
