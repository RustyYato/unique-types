//! a generic way to robustly reuse [`CounterValue`](crate::unique_indices::CounterValue)s

use core::{cell::RefCell, marker::PhantomData};
#[cfg(feature = "std")]
use std::sync::{Mutex, PoisonError, TryLockError};

/// A counter type which allows reusing identifiers via the `R: `[`Reuse`]
///
/// It uses `C: `[`Counter`](crate::unique_indices::Counter) as a source
/// of values
pub struct ReuseCounter<C, R> {
    counter: C,
    reuse: R,
}

// SAFETY: R will only yield values passed to it via reclaim and
// we otherwise forward toe C
// so this is trivially safe
unsafe impl<C: crate::unique_indices::Counter, R: Reuse<Value = C::Value>>
    crate::unique_indices::Counter for ReuseCounter<C, R>
{
    type Value = C::Value;

    const NEW: Self = Self {
        counter: C::NEW,
        reuse: R::NEW,
    };

    fn next_value(&self) -> Option<Self::Value> {
        self.reuse.extract().or_else(|| self.counter.next_value())
    }

    unsafe fn reclaim(&self, value: Self::Value) -> Result<(), Self::Value> {
        // SAFETY: the caller ensures that this value is legal to pass to reclaim
        // We first try to let the counter reclaim the value, since that will in general be cheaper
        // than doing it on the side. It is a single `cmpxchg` to decrement the counter
        // or setting the counter to false. And doing reclamation via our `reuse` will
        // in general grab a lock and may allocate.
        // So if usage patterns are well nested, then we will never need to actually hit
        // our `reuse`, but it they are not well nested, then we can use our reuse to
        // ensure that no keys are missed out on
        if let Err(value) = unsafe { self.counter.reclaim(value) } {
            self.reuse.reclaim(value)
        } else {
            Ok(())
        }
    }
}

/// A type that stores values to be reused later
///
/// # Safety
///
/// [`ReuseMut::extract_mut`] can only yield values that were passed to
/// [`ReuseMut::reclaim_mut`] or [`Reuse::reclaim`]
pub unsafe trait ReuseMut {
    /// The value this Reuse manages
    type Value;

    /// Create a new Reuse
    const NEW: Self;

    /// reclaim a value to be extracted
    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value>;

    /// extract a value from this reuse, this must be one that was passed to reclaim_mut or reclaim
    fn extract_mut(&mut self) -> Option<Self::Value>;
}

/// A type that stores values to be reused later
///
/// # Safety
///
/// [`Reuse::extract`] can only yield values that were passed to
/// [`ReuseMut::reclaim_mut`] or [`Reuse::reclaim`]
pub unsafe trait Reuse: ReuseMut {
    /// reclaim a value to be extracted
    fn reclaim(&self, value: Self::Value) -> Result<(), Self::Value>;

    /// extract a value from this reuse, this must be one that was passed to reclaim_mut or reclaim
    fn extract(&self) -> Option<Self::Value>;
}

/// Keeps up to `CAPACITY` elements in a stack
#[cfg(feature = "alloc")]
pub struct BoundedVec<T, const CAPACITY: usize>(alloc::vec::Vec<T>);

/// SAFETY: only yields values that were passed to [`ReuseMut::reclaim_mut`]
unsafe impl<T, const CAPACITY: usize> ReuseMut for BoundedVec<T, CAPACITY> {
    type Value = T;

    const NEW: Self = Self(alloc::vec::Vec::new());

    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value> {
        let v = &mut self.0;

        // if we haven't allocated yet, then reserve `CAPACITY` slots
        if v.capacity() == 0 {
            #[cold]
            #[inline(never)]
            fn alloc<T>(v: &mut alloc::vec::Vec<T>, capacity: usize) {
                v.reserve_exact(capacity);
            }

            alloc(v, CAPACITY)
        }

        // SAFETY: the vector's capacity is set once (just above) and never changed
        // after it is set. (since `Vec::push` only grows once v.len() == v.capacity())
        unsafe { core::hint::assert_unchecked(v.capacity() == CAPACITY) };

        if v.len() == v.capacity() {
            Err(value)
        } else {
            v.push(value);
            Ok(())
        }
    }

    fn extract_mut(&mut self) -> Option<Self::Value> {
        self.0.pop()
    }
}

#[cfg(feature = "std")]
// SAFETY: forwards to T
unsafe impl<T: ReuseMut> ReuseMut for Mutex<T> {
    type Value = T::Value;

    #[allow(clippy::declare_interior_mutable_const)]
    const NEW: Self = Self::new(T::NEW);

    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value> {
        self.get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .reclaim_mut(value)
    }

    fn extract_mut(&mut self) -> Option<Self::Value> {
        self.get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .extract_mut()
    }
}

#[cfg(feature = "std")]
// SAFETY: forwards to T
unsafe impl<T: ReuseMut> Reuse for Mutex<T> {
    fn reclaim(&self, value: Self::Value) -> Result<(), Self::Value> {
        match self.try_lock() {
            Ok(x) => x,
            Err(TryLockError::Poisoned(x)) => x.into_inner(),
            Err(TryLockError::WouldBlock) => return Err(value),
        }
        .reclaim_mut(value)
    }

    fn extract(&self) -> Option<Self::Value> {
        match self.try_lock() {
            Ok(x) => x,
            Err(TryLockError::Poisoned(x)) => x.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        }
        .extract_mut()
    }
}

// SAFETY: forwards to T
unsafe impl<T: ReuseMut> ReuseMut for RefCell<T> {
    type Value = T::Value;

    #[allow(clippy::declare_interior_mutable_const)]
    const NEW: Self = Self::new(T::NEW);

    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value> {
        self.get_mut().reclaim_mut(value)
    }

    fn extract_mut(&mut self) -> Option<Self::Value> {
        self.get_mut().extract_mut()
    }
}

// SAFETY: forwards to T
unsafe impl<T: ReuseMut> Reuse for RefCell<T> {
    fn reclaim(&self, value: Self::Value) -> Result<(), Self::Value> {
        match self.try_borrow_mut() {
            Ok(mut x) => x.reclaim_mut(value),
            Err(_) => Err(value),
        }
    }

    fn extract(&self) -> Option<Self::Value> {
        match self.try_borrow_mut() {
            Ok(mut x) => x.extract_mut(),
            Err(_) => None,
        }
    }
}

// SAFETY: pop can only yield values pushed onto the vec
unsafe impl<T> ReuseMut for alloc::vec::Vec<T> {
    type Value = T;

    const NEW: Self = Self::new();

    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value> {
        self.push(value);
        Ok(())
    }

    fn extract_mut(&mut self) -> Option<Self::Value> {
        self.pop()
    }
}

// SAFETY: always extracts None
unsafe impl<T> ReuseMut for PhantomData<T> {
    type Value = T;

    const NEW: Self = PhantomData;

    #[inline]
    fn reclaim_mut(&mut self, value: Self::Value) -> Result<(), Self::Value> {
        Err(value)
    }

    #[inline]
    fn extract_mut(&mut self) -> Option<Self::Value> {
        None
    }
}

// SAFETY: always extracts None
unsafe impl<T> Reuse for PhantomData<T> {
    #[inline]
    fn reclaim(&self, value: Self::Value) -> Result<(), Self::Value> {
        Err(value)
    }

    #[inline]
    fn extract(&self) -> Option<Self::Value> {
        None
    }
}
