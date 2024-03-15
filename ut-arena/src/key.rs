use ut_vec::UtVecElementIndex;

#[cfg(feature = "unique-types")]
use unique_types::UniqueToken;
#[cfg(feature = "unique-types")]
use ut_vec::UtIndex;

use crate::generation::{DefaultGeneration, Generation};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArenaKey<I = usize, G: Generation = DefaultGeneration> {
    index: I,
    generation: G::Filled,
}

impl<I, G: Generation> ArenaKey<I, G> {
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

pub unsafe trait ArenaIndex<O: ?Sized, G: Generation>: Copy {
    type UtIndex: UtVecElementIndex<O> + Copy;

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

#[cfg(feature = "unique-types")]
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

#[cfg(feature = "unique-types")]
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
