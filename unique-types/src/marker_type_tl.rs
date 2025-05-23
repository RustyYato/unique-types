//! This module allows you to create [`UniqueType`] values which all differ in types
//! and is checked at runtime. This is perfect for a long-lived [`UniqueType`] which
//! don't need to be used across threads.

use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use crate::UniqueType;

/// This is a marker type which owns the `T`, and when dropped will relinquish
/// it's ownership of `T`. You may have multiple [`TypeTlUt`] values, but they must all
/// have different types `T`.
///
/// NOTE: this type cannot be send/shared across threads, but it can be shared across threads
pub struct TypeTlUt<T: ?Sized + Any> {
    #[allow(clippy::type_complexity)]
    ty: PhantomData<*mut T>,
}

/// The token type for [`TypeTlUt`]
pub struct TypeTlUtToken<T: ?Sized> {
    #[allow(clippy::type_complexity)]
    ty: PhantomData<*mut T>,
}

impl<T: ?Sized> Copy for TypeTlUtToken<T> {}
impl<T: ?Sized> Clone for TypeTlUtToken<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Eq for TypeTlUtToken<T> {}
impl<T: ?Sized> PartialEq for TypeTlUtToken<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl<T: ?Sized> PartialOrd for TypeTlUtToken<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: ?Sized> Ord for TypeTlUtToken<T> {
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

impl<T: ?Sized> crate::TrivialToken for TypeTlUtToken<T> {
    const NEW: Self = TypeTlUtToken { ty: PhantomData };
}

// SAFETY: it is impossible to create two [`TypeTlUt<T>`] for the same `T`
unsafe impl<T: ?Sized + Any> UniqueType for TypeTlUt<T> {
    type Token = TypeTlUtToken<T>;

    fn token(&self) -> Self::Token {
        crate::TrivialToken::NEW
    }

    fn owns(&self, _token: &Self::Token) -> bool {
        true
    }
}

impl<T: ?Sized + Any> TypeTlUt<T> {
    fn id() -> TypeId {
        TypeId::of::<T>()
    }

    /// Try to create a new [`TypeTlUt`], and will never block, but may fail if there is already
    /// another [`TypeTlUt`] for the same `T`
    pub fn try_new() -> Option<Self> {
        if type_set::try_insert(Self::id()) {
            Some(Self { ty: PhantomData })
        } else {
            None
        }
    }

    /// Try to create a new [`TypeTlUt`], and will never block, but may fail if there is already
    /// another [`TypeTlUt`] for the same `T`.
    ///
    ///
    /// Will panic if another one is already created. Use [`TypeTlUt::try_new`] to avoid the panic.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        fn new_failed(s: &str) -> ! {
            panic!("Tried to create multiple `TypeTlUt` on the same thread for the type {s}, which is not allowed")
        }

        match Self::try_new() {
            Some(value) => value,
            None => new_failed(core::any::type_name::<T>()),
        }
    }
}

impl<T: ?Sized + Any> Drop for TypeTlUt<T> {
    fn drop(&mut self) {
        type_set::remove(Self::id());
    }
}

mod type_set {
    use core::any::TypeId;
    use core::cell::RefCell;
    use rustc_hash::FxHashSet;

    std::thread_local! {
        static SET: RefCell<FxHashSet<TypeId>> = const { RefCell::new(FxHashSet::with_hasher(rustc_hash::FxBuildHasher)) };
    }

    pub(super) fn try_insert(id: TypeId) -> bool {
        SET.with_borrow_mut(|set| set.insert(id))
    }

    pub(super) fn remove(id: TypeId) {
        SET.with_borrow_mut(|set| set.remove(&id));
    }
}

#[test]
fn test_try_new() {
    let x = TypeTlUt::<[i32; 1]>::try_new();
    let y = TypeTlUt::<[f32; 1]>::try_new();

    assert!(x.is_some());
    assert!(y.is_some());

    let x = TypeTlUt::<[i32; 1]>::try_new();
    let y = TypeTlUt::<[f32; 1]>::try_new();

    assert!(x.is_none());
    assert!(y.is_none());
}

#[test]
fn test_drop() {
    let x = TypeTlUt::<[i32; 2]>::try_new();
    let y = TypeTlUt::<[i32; 2]>::try_new();

    assert!(x.is_some());
    assert!(y.is_none());

    drop(x);

    let z = TypeTlUt::<[i32; 2]>::try_new();
    assert!(z.is_some());
}
