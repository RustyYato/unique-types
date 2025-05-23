//! This module allows you to create [`UniqueType`] values which all differ in types
//! and is checked at runtime. This is perfect for a long-lived [`UniqueType`] which
//! will be used across threads.

use core::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use crate::UniqueType;

/// This is a marker type which owns the `T`, and when dropped will relinquish
/// it's ownership of `T`. You may have multiple [`TypeUt`] values, but they must all
/// have different types `T`.
pub struct TypeUt<T: ?Sized + Any> {
    ty: PhantomData<fn() -> *mut T>,
}

/// The token type for [`TypeUt`]
pub struct TypeUtToken<T: ?Sized> {
    ty: PhantomData<fn() -> *mut T>,
}

impl<T: ?Sized> Copy for TypeUtToken<T> {}
impl<T: ?Sized> Clone for TypeUtToken<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Eq for TypeUtToken<T> {}
impl<T: ?Sized> PartialEq for TypeUtToken<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl<T: ?Sized> PartialOrd for TypeUtToken<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: ?Sized> Ord for TypeUtToken<T> {
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

impl<T: ?Sized> crate::TrivialToken for TypeUtToken<T> {
    const NEW: Self = TypeUtToken { ty: PhantomData };
}

// SAFETY: it is impossible to create two [`TypeUt<T>`] for the same `T`
unsafe impl<T: ?Sized + Any> UniqueType for TypeUt<T> {
    type Token = TypeUtToken<T>;

    fn token(&self) -> Self::Token {
        crate::TrivialToken::NEW
    }

    fn owns(&self, _token: &Self::Token) -> bool {
        true
    }
}

impl<T: ?Sized + Any> TypeUt<T> {
    fn id() -> TypeId {
        TypeId::of::<T>()
    }

    /// Create a new [`TypeUt`], and will block until one can be created for the given type
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        type_set::insert(Self::id());

        Self { ty: PhantomData }
    }

    /// Try to create a new [`TypeUt`], and will never block, but may fail if there is already
    /// another [`TypeUt`] for the same `T`
    pub fn try_new() -> Option<Self> {
        if type_set::try_insert(Self::id()) {
            Some(Self { ty: PhantomData })
        } else {
            None
        }
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized + Any> Default for TypeUt<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized + Any> Drop for TypeUt<T> {
    fn drop(&mut self) {
        // SAFETY: `TypeUt` is the sole owner of the typeid for `T`, by definition
        unsafe { type_set::remove(Self::id()) }
    }
}

#[cfg(all(feature = "std", not(feature = "exclusion-set")))]
mod type_set {
    use core::any::TypeId;
    use std::sync::{Condvar, Mutex, PoisonError};

    use rustc_hash::FxHashSet;

    static SET: Mutex<FxHashSet<TypeId>> =
        Mutex::new(FxHashSet::with_hasher(rustc_hash::FxBuildHasher));
    static CONDVAR: Condvar = Condvar::new();

    pub(super) fn try_insert(id: TypeId) -> bool {
        let mut set = SET.lock().unwrap_or_else(PoisonError::into_inner);
        set.insert(id)
    }

    pub(super) fn insert(id: TypeId) {
        let mut set = SET.lock().unwrap_or_else(PoisonError::into_inner);

        while !set.insert(id) {
            set = CONDVAR.wait(set).unwrap_or_else(PoisonError::into_inner);
        }
    }

    pub(super) unsafe fn remove(id: TypeId) {
        let mut set = SET.lock().unwrap_or_else(PoisonError::into_inner);
        set.remove(&id);
        CONDVAR.notify_all();
    }
}

#[cfg(feature = "exclusion-set")]
mod type_set {
    use core::any::TypeId;

    use exclusion_set::Set;

    static SET: Set<TypeId> = Set::new();

    pub(super) fn try_insert(id: TypeId) -> bool {
        SET.try_insert(id)
    }

    #[cfg(feature = "std")]
    pub(super) fn insert(id: TypeId) {
        SET.wait_to_insert(id);
    }

    pub(super) unsafe fn remove(id: TypeId) {
        // SAFETY: the caller ensures that this is the only thread that owns the given id
        unsafe { SET.remove(&id) };
    }
}

#[test]
#[cfg(feature = "std")]
fn test_new() {
    let _x = TypeUt::<[i32; 0]>::new();
    let _x = TypeUt::<[f32; 0]>::new();

    let x = TypeUt::<[i32; 0]>::try_new();
    let y = TypeUt::<[f32; 0]>::try_new();

    assert!(x.is_none());
    assert!(y.is_none());
}

#[test]
fn test_try_new() {
    let x = TypeUt::<[i32; 1]>::try_new();
    let y = TypeUt::<[f32; 1]>::try_new();

    assert!(x.is_some());
    assert!(y.is_some());

    let x = TypeUt::<[i32; 1]>::try_new();
    let y = TypeUt::<[f32; 1]>::try_new();

    assert!(x.is_none());
    assert!(y.is_none());
}

#[test]
fn test_drop() {
    let x = TypeUt::<[i32; 2]>::try_new();
    let y = TypeUt::<[i32; 2]>::try_new();

    assert!(x.is_some());
    assert!(y.is_none());

    drop(x);

    let z = TypeUt::<[i32; 2]>::try_new();
    assert!(z.is_some());
}
