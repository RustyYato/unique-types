//! This is a zero sized type which uses Rust's lifetimes to ensure that all values are unique

use core::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Invariant<'brand>(PhantomData<fn() -> *mut &'brand ()>);

/// A zero sized type which uses an invariant lifetime to ensure that all values
/// are distinct at compile time.
#[repr(transparent)]
pub struct LifetimeUt<'brand> {
    _brand: Invariant<'brand>,
}

/// The token type for [`LifetimeUt`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LifetimeUtToken<'brand> {
    _brand: Invariant<'brand>,
}

impl LifetimeUt<'_> {
    /// Calls the closure with the given lifetime
    pub fn with<R>(f: impl FnOnce(LifetimeUt<'_>) -> R) -> R {
        // SAFETY:  because the closure is generic over the brand, and LifetimeUt
        // is invariant in 'brand. The closure cannot confuse two different LifetimeUt values
        // as the same type.
        f(unsafe { LifetimeUt::new_unchecked() })
    }

    /// # Safety
    ///
    /// This should not be used by downstream crates except through the [`lifetime!()`] macro
    #[doc(hidden)]
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _brand: Invariant(PhantomData),
        }
    }
}

// SAFETY: This type ensures that there can't be two instances of the same type
// via lifetime tricks. So it's not possible to even call the `no_duplicates` function
// let alone trigger any asserts in the function.
unsafe impl<'brand> crate::UniqueType for LifetimeUt<'brand> {
    type Token = LifetimeUtToken<'brand>;

    fn token(&self) -> Self::Token {
        LifetimeUtToken {
            _brand: Invariant(PhantomData),
        }
    }

    fn owns(&self, _token: &Self::Token) -> bool {
        true
    }
}

// SAFETY: This type ensures that there can't be two instances of the same type
// and since all tokens have the same invariant lifetime as the [`LifetimeUt`]
// that created them, it's only possible to call `owns` with the value that
// created the token.
unsafe impl crate::UniqueToken for LifetimeUt<'_> {}

impl crate::TrivialToken for LifetimeUtToken<'_> {
    const NEW: Self = Self {
        _brand: Invariant(PhantomData),
    };
}

/// Creates a type with a unique lifetime
///
/// ```compile_fail,E0597
/// # use unique_types::{UniqueType, unique_lifetime};
/// unique_lifetime!(a);
/// unique_lifetime!(b);
/// assert_eq!(a.token(), b.token());
/// ```
#[macro_export]
macro_rules! unique_lifetime {
    ($name:ident) => {
        let $name = ();
        let dropvalue = $crate::lifetime::DropValue::new(&$name);
        // SAFETY: dropvalue lives until the end of the block, and any subsequent calls
        // to unique_lifetime will live for a strictly shorter lifetime
        // This is because LifetimeUt is invariant in 'brand and DropValue is invariant in 'brand
        // and DropValue implements drop
        // This is because DropValue implements drop, it ensures that the lifetime is "used" at the
        // end of the bloc,
        let $name = unsafe { $crate::lifetime::LifetimeUt::new_unchecked() };
        dropvalue.brand(&$name);
    };
}

#[doc(hidden)]
pub struct DropValue<'brand> {
    _brand: Invariant<'brand>,
}

impl Drop for DropValue<'_> {
    fn drop(&mut self) {}
}

impl<'brand> DropValue<'brand> {
    pub const fn new(_: &'brand ()) -> Self {
        Self {
            _brand: Invariant(PhantomData),
        }
    }

    pub const fn brand(&self, _lifetime: &LifetimeUt<'brand>) {}
}
