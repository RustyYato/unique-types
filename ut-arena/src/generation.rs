use core::{fmt, hash::Hash};

pub unsafe trait Generation: Copy + Ord + Hash + core::fmt::Debug {
    const EMPTY: Self;

    type TryEmptyError;
    type Filled: Copy + Ord + Hash + core::fmt::Debug;

    unsafe fn fill(self) -> Self;

    unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError>;

    unsafe fn to_filled(self) -> Self::Filled;

    fn matches(self, filled: Self::Filled) -> bool;

    fn write_mismatch(
        self,
        filled: Self::Filled,
        index: usize,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    fn is_empty(self) -> bool;

    #[inline]
    fn is_filled(self) -> bool {
        !self.is_empty()
    }
}

type DefaultGenerationInner = gsize;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultGeneration(DefaultGenerationInner);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultGenerationFilled(<DefaultGenerationInner as Generation>::Filled);

unsafe impl Generation for DefaultGeneration {
    const EMPTY: Self = Self(DefaultGenerationInner::EMPTY);

    type TryEmptyError = <DefaultGenerationInner as Generation>::TryEmptyError;
    type Filled = DefaultGenerationFilled;

    #[inline]
    unsafe fn fill(self) -> Self {
        Self(self.0.fill())
    }

    #[inline]
    unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
        self.0.try_empty().map(Self)
    }

    unsafe fn to_filled(self) -> Self::Filled {
        DefaultGenerationFilled(self.0.to_filled())
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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoGeneration(bool);

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

macro_rules! prim {
    ($name:ident $name_filled:ident $inner:ident $filled_inner:ident) => {
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name_filled(core::num::$filled_inner);

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

        unsafe impl Generation for $name {
            const EMPTY: Self = Self(0);

            type TryEmptyError = ();
            type Filled = $name_filled;

            #[inline]
            unsafe fn fill(self) -> Self {
                Self(self.0 | 1)
            }

            #[inline]
            unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
                self.0.checked_add(1).map(Self).ok_or(())
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
                $name_filled(core::num::$filled_inner::new_unchecked(self.0))
            }

            #[inline]
            fn is_empty(self) -> bool {
                self.0 & 1 == 0
            }
        }
    };
}

macro_rules! prim_wrapping {
    ($name:ident $name_filled:ident $inner:ident $filled_inner:ident) => {
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);
        #[repr(transparent)]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name_filled(core::num::$filled_inner);

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

        unsafe impl Generation for $name {
            const EMPTY: Self = Self(0);

            type TryEmptyError = core::convert::Infallible;
            type Filled = $name_filled;

            #[inline]
            unsafe fn fill(self) -> Self {
                Self(self.0 | 1)
            }

            #[inline]
            unsafe fn try_empty(self) -> Result<Self, Self::TryEmptyError> {
                Ok(Self(self.0.wrapping_add(1)))
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
                $name_filled(core::num::$filled_inner::new_unchecked(self.0))
            }

            #[inline]
            fn is_empty(self) -> bool {
                self.0 & 1 == 0
            }
        }
    };
}

prim!(g8 FilledG8 u8 NonZeroU8);
prim!(g16 FilledG16 u16 NonZeroU16);
prim!(g32 FilledG32 u32 NonZeroU32);
prim!(g64 FilledG64 u64 NonZeroU64);
prim!(g128 FilledG128 u128 NonZeroU128);

prim!(gsize FilledGsize usize NonZeroUsize);

prim_wrapping!(gw8  FilledGw8 u8 NonZeroU8);
prim_wrapping!(gw16 FilledGw16 u16 NonZeroU16);
prim_wrapping!(gw32 FilledGw32 u32 NonZeroU32);
prim_wrapping!(gw64 FilledGw64 u64 NonZeroU64);
prim_wrapping!(gw128 FilledGw128 u128 NonZeroU128);

prim_wrapping!(gwsize FilledGwsize usize NonZeroUsize);
