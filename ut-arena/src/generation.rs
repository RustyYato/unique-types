//! generations are used for two purposes, to track if a given slot is filled and to harden against
//! the ABA-problem
//!
//! see [`Generation`] for details

use core::{fmt, hash::Hash};

/// [`Generation`] is used to track the initializatoin state and number of removals of a slot
///
/// Here's an example of how the lifetime of a slot can be modeled using [`Generation`]
///
/// * every slot starts off empty, so [`Generation::EMPTY`] is used as the generation
/// * when you fill an empty slot, the generation is updated via [`Generation::fill`]
///     * this always succeeds, since you should always be able to fill an empty slot
/// * when you need to remove a value from a slot, you should call [`Generation::try_empty`]
///     * if this succeeds, then the new generation should be used for the slot
///     * if this fails, then the slot's generation shoud be set to [`Generation::EMPTY`] and
///       the slot should be discarded, never to be filled again
/// * when creating a key for a filled slot, you should call [`Generation::to_filled`]
///     * this creates a more optimizated representation of the generation for keys
///       for example, for `NoGeneration` this is just a `()`
/// * you can check if a key's generation matches a slot's generation via [`Generation::matches`]
///     * and [`Generation::write_mismatch`] writes the error message in case of these don't match
/// * is_empty, and is_filled can be used to check if the slot for this generation is filled or
///       empty
///
/// # Safety
///
/// Your generation should pass all these tests for all valid instances of your type
/// that are reachable from `Self::EMPTY`, and calls to `fill` and `try_empty`
///
/// ```
/// # use ut_arena::generation::Generation;
/// fn test_generation<G: Generation>(g: G, filled: G::Filled) {
///     assert!( G::EMPTY.is_empty() );
///     assert!( g.is_empty() != g.is_filled() );
///
///     if g.is_empty() {
///         unsafe { assert!(g.fill().is_filled()) }
///     } else if let Ok(g) = unsafe { g.try_empty() } {
///         assert!(g.is_empty());
///     }
///
///     if g.matches(filled) {
///         assert!(g.is_filled())
///     }
/// }
/// ```
pub unsafe trait Generation: Copy + Ord + Hash + core::fmt::Debug {
    /// The initial generation, which is guaranteed to be empty
    const EMPTY: Self;

    /// If [`Generation::try_empty`] can fail, this should be ()
    /// otherwise this should be [`core::convert::Infallible`]
    type TryEmptyError: Copy;

    /// The filled representation of the [`Generation`]
    type Filled: Copy + Ord + Hash + core::fmt::Debug;

    /// Get the next generation
    ///
    /// # Safety
    ///
    /// The generation must currently be empty
    unsafe fn fill(self) -> Self;

    /// Get the next generation
    ///
    /// May return an error if the genration has been exhausted
    ///
    /// # Safety
    ///
    /// The generation must currently be filled
    unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError>;

    /// Convert a filled generation to [`Generation::Filled`]
    ///
    /// # Safety
    ///
    /// The generation must be filled
    unsafe fn to_filled(self) -> Self::Filled;

    /// Check if a generation matches the filled generation
    fn matches(self, filled: Self::Filled) -> bool;

    /// Writes the error of a failed `matches`
    fn write_mismatch(
        self,
        filled: Self::Filled,
        slot_index: usize,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// Check if the generation is in the empty variant
    fn is_empty(self) -> bool;

    /// Check if the generation is in the filled variant
    #[inline]
    fn is_filled(self) -> bool {
        !self.is_empty()
    }
}

type DefaultGenerationInner = gsize;

/// The default generation type, currently just a thin wrapper around [`gsize`]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct DefaultGeneration(DefaultGenerationInner);

/// The default generation's filled type, currently just a thin wrapper around [`FilledGsize`]'
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct DefaultGenerationFilled(<DefaultGenerationInner as Generation>::Filled);

#[cfg(kani)]
#[kani::proof]
fn proof_default_generation() {
    let g = kani::any::<DefaultGeneration>();
    let f = kani::any::<DefaultGenerationFilled>();
    test_generation(g, f);
}

/// SAFETY: defers to `gsize`
unsafe impl Generation for DefaultGeneration {
    const EMPTY: Self = Self(DefaultGenerationInner::EMPTY);

    type TryEmptyError = <DefaultGenerationInner as Generation>::TryEmptyError;
    type Filled = DefaultGenerationFilled;

    #[inline]
    unsafe fn fill(self) -> Self {
        // SAFETY:ensured by caller
        Self(unsafe { self.0.fill() })
    }

    #[inline]
    unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
        // SAFETY:ensured by caller
        unsafe { self.0.try_empty() }.map(Self)
    }

    unsafe fn to_filled(self) -> Self::Filled {
        // SAFETY:ensured by caller
        DefaultGenerationFilled(unsafe { self.0.to_filled() })
    }

    #[inline]
    fn matches(self, filled: Self::Filled) -> bool {
        self.0.matches(filled.0)
    }

    #[inline]
    fn write_mismatch(
        self,
        filled: Self::Filled,
        index: usize,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.0.write_mismatch(filled.0, index, f)
    }

    #[inline]
    fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    fn is_filled(self) -> bool {
        self.0.is_filled()
    }
}

/// The generation type to ignore ABA issues
///
/// This only discriminates between filled and empty slots, and nothing more
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub struct NoGeneration(bool);

#[cfg(kani)]
#[kani::proof]
fn proof_no_generation() {
    let g = kani::any::<NoGeneration>();
    let f = kani::any::<()>();
    test_generation(g, f);
}

// SAFETY: see test_no_generation for passing test
unsafe impl Generation for NoGeneration {
    const EMPTY: Self = Self(false);

    type TryEmptyError = core::convert::Infallible;
    type Filled = ();

    #[inline]
    unsafe fn fill(self) -> Self {
        Self(true)
    }

    #[inline]
    unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
        Ok(Self(false))
    }

    #[inline]
    unsafe fn to_filled(self) -> Self::Filled {}

    #[inline]
    fn matches(self, (): Self::Filled) -> bool {
        self.0
    }

    fn write_mismatch(
        self,
        (): Self::Filled,
        index: usize,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "tried to access an empty slot at index {index}, which is illegal"
        )
    }

    #[inline]
    fn is_empty(self) -> bool {
        !self.0
    }
}

macro_rules! prim_impl {
    (ty wrapping) => {
        core::convert::Infallible
    };
    (ty saturating) => {
        ()
    };
    (fn wrapping($self:ident, $Self:ident)) => {
        Ok($Self($self.0.wrapping_add(1)))
    };
    (fn saturating($self:ident, $Self:ident)) => {
        $self.0.checked_add(1).map($Self).ok_or(())
    };
}

macro_rules! prim {
    (
        $(#[$meta_name:meta])*
        $name:ident

        $(#[$meta_filled:meta])*
        $name_filled:ident

        $inner:ident
        $filled_inner:ident

        $kind:ident
    ) => {
        $(#[$meta_name])*
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(kani, derive(kani::Arbitrary))]
        pub struct $name($inner);
        $(#[$meta_filled])*
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name_filled(core::num::$filled_inner);

        const _: () = {
            #[cfg(kani)]
            #[kani::proof]
            fn $name() {
                let g = kani::any::<$name>();
                let f = kani::any::<$name_filled>();
                test_generation(g, f);
            }
        };

        #[cfg(kani)]
        impl kani::Arbitrary for $name_filled {
            fn any() -> Self {
                let inner = kani::any::<core::num::$filled_inner>();
                kani::assume(inner.get() & 1 == 1);
                Self(inner)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl core::fmt::Debug for $name_filled {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        // SAFETY: see proof above in const _: ()
        unsafe impl Generation for $name {
            const EMPTY: Self = Self(0);

            type TryEmptyError = prim_impl!(ty $kind);
            type Filled = $name_filled;

            #[inline]
            unsafe fn fill(self) -> Self {
                Self(self.0 | 1)
            }

            #[inline]
            unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
                prim_impl!(fn $kind(self, Self))
            }

            #[inline]
            fn matches(self, filled: Self::Filled) -> bool {
                self.0 == filled.0.get()
            }

            fn write_mismatch(
                self,
                filled: Self::Filled,
                index: usize,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result {
                write!(
                    f,
                    "tried to access arena with an expired key at index {index} with generation: {filled:?}, but expected generation: {self:?}"
                )
            }

            #[inline]
            unsafe fn to_filled(self) -> Self::Filled {
                // SAFETY: all filled generations have the least significant bit set, so mut be
                // non-zero
                $name_filled(unsafe { core::num::$filled_inner::new_unchecked(self.0) })
            }

            #[inline]
            fn is_empty(self) -> bool {
                self.0 & 1 == 0
            }
        }
    };
}

macro_rules! prim_saturating {
    (
        $(#[$meta_name:meta])*
        $name:ident

        $(#[$meta_filled:meta])*
        $name_filled:ident

        $inner:ident
        $filled_inner:ident
    ) => {
        prim! {
            $(#[$meta_name])*
            $name
            $(#[$meta_filled])*
            $name_filled

            $inner
            $filled_inner

            saturating
        }
    };
}

macro_rules! prim_wrapping {
    (
        $(#[$meta_name:meta])*
        $name:ident

        $(#[$meta_filled:meta])*
        $name_filled:ident

        $inner:ident
        $filled_inner:ident
    ) => {
        prim! {
            $(#[$meta_name])*
            $name
            $(#[$meta_filled])*
            $name_filled

            $inner
            $filled_inner

            wrapping
        }
    };
}

prim_saturating!(
    /// A 8-bit saturating generation
    g8
    /// The key version of [`g8`]
    FilledG8
    u8 NonZeroU8
);
prim_saturating!(
    /// A 16-bit saturating generation
    g16
    /// The key version of [`g16`]
    FilledG16
    u16 NonZeroU16
);
prim_saturating!(
    /// a 32-bit saturating generation
    g32
    /// The key version of [`g32`]
    FilledG32
    u32 NonZeroU32
);
prim_saturating!(
    /// 64-bit saturating generation
    g64
    /// The key version of [`g64`]
    FilledG64
    u64
    NonZeroU64
);
prim_saturating!(
    /// The 128-bit saturating generation
    g128
    /// The key version of [`g128`]
    FilledG128
    u128
    NonZeroU128
);

prim_saturating!(
    /// A pointer sized saturating generation
    gsize
    /// The key version of [`gsize`]
    FilledGsize
    usize
    NonZeroUsize
);

prim_wrapping!(
    /// A 8-bit wrapping generation
    gw8
    /// The key version of [`gw8`]
    FilledGw8
    u8 NonZeroU8
);
prim_wrapping!(
    /// A 16-bit wrapping generation
    gw16
    /// The key version of [`gw16`]
    FilledGw16
    u16 NonZeroU16
);
prim_wrapping!(
    /// a 32-bit wrapping generation
    gw32
    /// The key version of [`gw32`]
    FilledGw32
    u32 NonZeroU32
);
prim_wrapping!(
    /// 64-bit wrapping generation
    gw64
    /// The key version of [`gw64`]
    FilledGw64
    u64
    NonZeroU64
);
prim_wrapping!(
    /// The 128-bit wrapping generation
    gw128
    /// The key version of [`gw128`]
    FilledGw128
    u128
    NonZeroU128
);

prim_wrapping!(
    /// A pointer sized wrapping generation
    gwsize
    /// The key version of [`gwsize`]
    FilledGwsize
    usize
    NonZeroUsize
);

#[cfg(kani)]
fn test_generation<G: Generation>(g: G, filled: G::Filled) {
    assert!(G::EMPTY.is_empty());
    assert!(g.is_empty() != g.is_filled());

    if g.is_empty() {
        // SAFETY: ^^^ g is empty
        unsafe { assert!(g.fill().is_filled()) }
    } else {
        // SAFETY: g is currently in the filled state
        assert!(g.matches(unsafe { g.to_filled() }));
        // SAFETY: g is currently in the filled state
        if let Ok(g) = unsafe { g.try_empty() } {
            assert!(g.is_empty())
        }
    }

    if g.matches(filled) {
        assert!(g.is_filled())
    }
}
