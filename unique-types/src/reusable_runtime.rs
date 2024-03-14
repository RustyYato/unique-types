//! Represents a [`UniqueType`] which tries to aquire a unique value at runtime, and will reclaim
//! the value it aquired after it is done with it

use core::{hash::Hash, marker::PhantomData};

use crate::{
    unique_indices::{Counter, CounterRef, GlobalCounter},
    UniqueType,
};

/// A [`UniqueType`] which checks at runtime if it is unique
/// and will reclaim the value used on drop
pub struct ReuseRuntimeUt<C: CounterRef = GlobalCounter> {
    value: C::Value,
    _ty_traits: PhantomData<C::TypeTraits>,
}

/// The token for [`ReuseRuntimeUt`]
pub struct ReuseRuntimeUtToken<C: CounterRef> {
    value: C::Value,
    _ty_traits: PhantomData<C::TypeTraits>,
}

impl<C: CounterRef> Copy for ReuseRuntimeUtToken<C> {}
impl<C: CounterRef> Clone for ReuseRuntimeUtToken<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: CounterRef> Eq for ReuseRuntimeUtToken<C> {}
impl<C: CounterRef> PartialEq for ReuseRuntimeUtToken<C> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<C: CounterRef> PartialOrd for ReuseRuntimeUtToken<C> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: CounterRef> Ord for ReuseRuntimeUtToken<C> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<C: CounterRef> Hash for ReuseRuntimeUtToken<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl ReuseRuntimeUt {
    /// Create a new [`ReuseRuntimeUt`] based on the [`GlobalCounter`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_counter()
    }

    /// Try to create a new [`ReuseRuntimeUt`] based on the [`GlobalCounter`]
    pub fn try_new() -> Option<Self> {
        Self::try_with_counter()
    }
}

impl<C: CounterRef> ReuseRuntimeUt<C> {
    /// Create a new [`ReuseRuntimeUt`] based on the given counter
    pub fn with_counter() -> Self {
        Self::try_with_counter().expect("Tried to create a new RuntimeUt from an exhausted counter")
    }

    /// Create a new [`ReuseRuntimeUt`] based on the given counter
    pub fn try_with_counter() -> Option<Self> {
        Some(Self {
            _ty_traits: PhantomData,
            value: C::with(Counter::next_value)?,
        })
    }
}

// SAFETY: CounterRef guarantees that only one value
unsafe impl<C: CounterRef> UniqueType for ReuseRuntimeUt<C> {
    type Token = ReuseRuntimeUtToken<C>;

    fn token(&self) -> Self::Token {
        ReuseRuntimeUtToken {
            _ty_traits: self._ty_traits,
            value: self.value,
        }
    }

    fn owns(&self, token: &Self::Token) -> bool {
        self.value == token.value
    }
}

impl<C: CounterRef<Value = ()>> crate::TrivialToken for ReuseRuntimeUtToken<C> {
    const NEW: Self = Self {
        value: (),
        _ty_traits: PhantomData,
    };
}

impl<C: CounterRef> Drop for ReuseRuntimeUt<C> {
    fn drop(&mut self) {
        // SAFETY:
        // * the value will not be used since we are in Drop
        // * This ReuseRuntimeUt owns the value
        // * C::with ensures that this is the same counter as in try_with_counter
        C::with(|counter| unsafe {
            let _ = counter.reclaim(self.value);
        })
    }
}
