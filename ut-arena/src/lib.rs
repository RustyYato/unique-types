#![no_std]
#![forbid(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    unsafe_op_in_unsafe_fn,
    missing_docs,
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::alloc_instead_of_core
)]

//! # ut-arena
//!
//! Provide some arenas with a varying level of ABA-hardening, bounds
//! check elision, and performance characteristics.
//!
//! ## ABA-hardening
//!
//! The [ABA problem](https://en.wikipedia.org/wiki/ABA_problem) is where
//! you expect a key to exist, but it has since been removed and replaced
//! so it looks like it still exists, but it no longer does
//!
//! To fix this, we use the types in [`generation`] to track how many removals
//! have happened, and if the number of removals doesn't match your key, then
//! your key is invalid.
//!
//! More concretely,
//! ```
//! use ut_arena::generic_sparse::GenericSparseArena as Arena;
//! use ut_arena::key::ArenaKey;
//!
//! let mut arena = Arena::<char>::new();
//! let key_a: ArenaKey = arena.insert('a');
//! let key_b: ArenaKey = arena.insert('b');
//!
//! assert_eq!(arena.remove(key_a), 'a');
//!
//! // this will be inserted into the same slot that 'a' was inserted into
//! let key_c: ArenaKey = arena.insert('c');
//!
//! assert_eq!(key_a.index(), key_c.index());
//!
//! // see, even though key_a has the same index as key_c
//! // using key_a will still fail to access any elements
//! // this is because after removing key_a, and inserting
//! // 'c', the generation of the slot has been updated
//! // and no longer matches the generation that key_a was
//! // created with
//! assert_eq!(arena.try_remove(key_a), None);
//! ```
//!
//! There are three main strategies for ABA-hardening
//!
//! 1. increment a `uN` counter and if it ever reaches `uN::MAX`, then that slot is
//!    exhausted. This slot will never contain any new elements in it.
//!    This is handled by the `gN` types in [`generation`], such as [`g32`](generation::g32)
//!
//! 2. increment a `uN` counter and if it ever reaches `uN::MAX`, just wrap around back to 0
//!    NOTE: this looses the guarantee that all ArenaKeys are unique, but also allows reusing
//!    slots endlessly. So you are less likely to end up with leaks if you use a small N.
//!    This is handled by the `gwN` types in [`generation`], such as [`gw32`](generation::gw32)
//!
//! 3. flip a bool to indicate if the slot is empty or not.
//!    NOTE: This doesn't handle the ABA problem at all. But can be useful if that's not actually
//!    a problem for your domain. This is implemented by [`NoGeneration`](generation::NoGeneration)
//!
//! The default strategy ([`DefaultGeneration`](generation::DefaultGeneration)) currently uses
//! [`gsize`](generation::gsize) as the backing generation type, but this may be changed in the future.
//! However it will always guarantee that all [`ArenaKey`](key::ArenaKey)s you get after insertion are
//! unique. Thus avoiding the ABA-problem entirely.
//!
//! Practically, there isn't much difference between [`g32`](generation::g32) and
//! [`gw32`](generation::gw32), since it is unlikely that you will ever exhaust a 32-bits for a
//! single slot. So having the guarantee is quite nice. However for smaller bits, it is possible
//! and quite likely that you will exhaust the bits, so pick carefully.
//!
//! ## Bounds check elision
//!
//! If you enable the [`unique-types`](unique_types) feature and use the owners from
//! [`unique-types`](unique_types), and [`UtIndex`](ut_vec::UtIndex) inside arena keys, then all
//! bounds checks will be eliminated. see [`ut-vec`] for more details here.
//!
//! ## Arena Types
//!
//! This crate provides two distinct arena types with different trade offs
//!
//! ### sparse arenas
//!
//! Sparse arenas don't store all elements contiguously, or track how many elements they have.
//!
//! They have a very fast access, insertion, and removal. All O(1) performance cost.
//! Their memory footprint is also the same as `Vec<T>` + 1 usize if your elements are at least as
//! large as `usize`. Making them extremely memory efficient.
//!
//! The cost: iteration speed scales with the number of slots, not the number of
//! elements. This is particularly bad if you iterate over the arena after removing many elements,
//! since it will still iterate over all the empty slots left.
//!
//! This makes them ideal if ...
//! * you can keep track of the keys and don't need iteration
//! * there is a small upper bounds to how many elements you need to track
//! * you don't do mass removals of elements
//!
//! ### dense arenas
//!
//! Dense arenas store elements contiguously and can track how many elements they have.
//!
//! They still retain O(1) access, insertion, and removal, but the constant factors are a bit
//! larger than Sparse Arenas because there is a double indirection between the key and the
//! element.
//!
//! Basically, dense arenas use a sparse arena to map keys -> indices in a [`Vec`](alloc::vec::Vec), which
//! is a double indirection.
//!
//! But this is a small cost to pay for the fastest iteration speed you can ask for.
//! Iteration is bounded by the number of actual elements you have, not how many slots you have.
//!
//! This makes dense arenas ideal when iteration speed is required.

extern crate alloc;

pub mod dense_tracker;
pub mod generic_dense;
pub mod generic_sparse;

pub mod generation;
pub mod internal_index;
pub mod key;

pub mod dense_slab;
pub mod slab;

pub mod dense_slotmap;
pub mod slotmap;

mod key_hash;

mod seal {
    pub trait Seal {}
}
