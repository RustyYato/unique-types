#![allow(clippy::declare_interior_mutable_const)]

//! this is a helper module to implement counters that always yield unique values

use core::{
    cell::Cell,
    hash::Hash,
    num::{NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8},
    sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering},
};

/// A reference to a [`Counter`]
///
/// # Safety
///
/// The same counter must be passed to the closure in with each time with is called
pub unsafe trait CounterRef {
    /// The counter this value references
    type Counter: Counter<Value = Self::Value>;
    /// The counter value the counter produces
    type Value: Copy + Ord + Hash;

    /// Access the counter reference
    fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T;
}

/// A global [`CounterRef`] that yields [`NonZeroU64`]
pub struct GlobalCounter;

// SAFETY: GLOBAL_COUNTER is private to with and GLOBAL_COUNTER is always passed to the closure
unsafe impl CounterRef for GlobalCounter {
    type Counter = AtomicCounterU64;
    type Value = NonZeroU64;

    fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
        static GLOBAL_COUNTER: AtomicCounterU64 = Counter::NEW;
        f(&GLOBAL_COUNTER)
    }
}

/// A counter type is one where you can call next_value until there are no more values to yield
///
/// # Safety
///
/// Every value yielded by Counter should be unique
///
/// And all copies of the value must compare equal
pub unsafe trait Counter {
    /// The value yielded by [`Counter::next_value`]
    type Value: Copy + Ord + Hash;
    /// A type which implements all the traits needed to ensure that the
    /// value isn't exposed to threads if it shouldn't be
    type TypeTraits;

    /// Create a new counter
    const NEW: Self;

    /// Get the next value from the counter
    fn next_value(&self) -> Option<Self::Value>;
}

/// A value yielded by a counter
pub trait CounterValue {
    /// The thread-unsafe counter
    type CellCounter: Counter<Value = Self>;
    /// The thread-safe counter
    type AtomicCounter: Counter<Value = Self> + Send + Sync;
}

/// A thread-unsafe [`Counter`]
pub struct CellCounter<T>(Cell<T>);

// SAFETY: next_value only returns Some once
unsafe impl Counter for CellCounter<bool> {
    type Value = ();
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(true));

    fn next_value(&self) -> Option<Self::Value> {
        if self.0.replace(false) {
            Some(())
        } else {
            None
        }
    }
}

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for CellCounter<u8> {
    type Value = NonZeroU8;
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = self.0.get().checked_add(1)?;
        self.0.set(x);
        Some(NonZeroU8::new(x).unwrap())
    }
}

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for CellCounter<u16> {
    type Value = NonZeroU16;
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = self.0.get().checked_add(1)?;
        self.0.set(x);
        Some(NonZeroU16::new(x).unwrap())
    }
}

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for CellCounter<u32> {
    type Value = NonZeroU32;
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = self.0.get().checked_add(1)?;
        self.0.set(x);
        Some(NonZeroU32::new(x).unwrap())
    }
}

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for CellCounter<u64> {
    type Value = NonZeroU64;
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = self.0.get().checked_add(1)?;
        self.0.set(x);
        Some(NonZeroU64::new(x).unwrap())
    }
}

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for CellCounter<u128> {
    type Value = NonZeroU128;
    type TypeTraits = *mut ();

    const NEW: Self = Self(Cell::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = self.0.get().checked_add(1)?;
        self.0.set(x);
        Some(NonZeroU128::new(x).unwrap())
    }
}

/// A thread-safe counter for [`()`]
pub struct AtomicCounterBool(AtomicBool);

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for AtomicCounterBool {
    type Value = ();
    type TypeTraits = *mut ();

    const NEW: Self = Self(AtomicBool::new(false));

    fn next_value(&self) -> Option<Self::Value> {
        if self
            .0
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            Some(())
        } else {
            None
        }
    }
}

/// A thread-safe counter for [`NonZeroU8`]
pub struct AtomicCounterU8(AtomicU8);

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for AtomicCounterU8 {
    type Value = NonZeroU8;
    type TypeTraits = *mut ();

    const NEW: Self = Self(AtomicU8::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = 1 + self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .ok()?;

        // SAFETY: fetch_update will only return Ok if the closure didn't return None
        // and it will return the old value (before the closure was run), so adding 1 to it
        // will yield a non-zero value
        Some(unsafe { NonZeroU8::new_unchecked(x) })
    }
}

/// A thread-safe counter for [`NonZeroU16`]
pub struct AtomicCounterU16(AtomicU16);

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for AtomicCounterU16 {
    type Value = NonZeroU16;
    type TypeTraits = *mut ();

    const NEW: Self = Self(AtomicU16::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = 1 + self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .ok()?;

        // SAFETY: fetch_update will only return Ok if the closure didn't return None
        // and it will return the old value (before the closure was run), so adding 1 to it
        // will yield a non-zero value
        Some(unsafe { NonZeroU16::new_unchecked(x) })
    }
}

/// A thread-safe counter for [`NonZeroU32`]
pub struct AtomicCounterU32(AtomicU32);

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for AtomicCounterU32 {
    type Value = NonZeroU32;
    type TypeTraits = *mut ();

    const NEW: Self = Self(AtomicU32::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = 1 + self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .ok()?;

        // SAFETY: fetch_update will only return Ok if the closure didn't return None
        // and it will return the old value (before the closure was run), so adding 1 to it
        // will yield a non-zero value
        Some(unsafe { NonZeroU32::new_unchecked(x) })
    }
}

/// A thread-safe counter for [`NonZeroU64`]
pub struct AtomicCounterU64(AtomicU64);

// SAFETY: next_value always increments itself so it can never return the same value multiple times
unsafe impl Counter for AtomicCounterU64 {
    type Value = NonZeroU64;
    type TypeTraits = *mut ();

    const NEW: Self = Self(AtomicU64::new(0));

    fn next_value(&self) -> Option<Self::Value> {
        let x = 1 + self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .ok()?;

        // SAFETY: fetch_update will only return Ok if the closure didn't return None
        // and it will return the old value (before the closure was run), so adding 1 to it
        // will yield a non-zero value
        Some(unsafe { NonZeroU64::new_unchecked(x) })
    }
}

impl CounterValue for () {
    type CellCounter = CellCounter<bool>;
    type AtomicCounter = AtomicCounterBool;
}

impl CounterValue for NonZeroU8 {
    type CellCounter = CellCounter<u8>;
    type AtomicCounter = AtomicCounterU8;
}

impl CounterValue for NonZeroU16 {
    type CellCounter = CellCounter<u16>;
    type AtomicCounter = AtomicCounterU16;
}

impl CounterValue for NonZeroU32 {
    type CellCounter = CellCounter<u32>;
    type AtomicCounter = AtomicCounterU32;
}

impl CounterValue for NonZeroU64 {
    type CellCounter = CellCounter<u64>;
    type AtomicCounter = AtomicCounterU64;
}
