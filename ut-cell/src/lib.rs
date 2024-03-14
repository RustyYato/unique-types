#![no_std]

use core::{cell::UnsafeCell, mem};

use unique_types::{TrivialToken, UniqueType};

#[doc(hidden)]
pub mod load_all;

impl<T: ?Sized + UniqueType> CellOwner for T {}
pub trait CellOwner: UniqueType {
    fn get<'a, T: ?Sized>(&'a self, cell: &'a UtCell<T, Self>) -> &'a T {
        cell.load(self)
    }

    fn get_mut<'a, T: ?Sized>(&'a mut self, cell: &'a UtCell<T, Self>) -> &'a mut T {
        cell.load_mut(self)
    }

    fn get_mut2<'a, T: ?Sized, U: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
    ) -> (&'a mut T, &'a mut U) {
        load_all!(
            self =>
            let a = a;
            let b = b;
        );
        (a, b)
    }

    fn get_mut3<'a, T: ?Sized, U: ?Sized, V: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
    ) -> (&'a mut T, &'a mut U, &'a mut V) {
        load_all!(
            self =>
            let a = a;
            let b = b;
            let c = c;
        );
        (a, b, c)
    }

    fn get_mut4<'a, T: ?Sized, U: ?Sized, V: ?Sized, X: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
        d: &'a UtCell<X, Self>,
    ) -> (&'a mut T, &'a mut U, &'a mut V, &'a mut X) {
        load_all!(
            self =>
            let a = a;
            let b = b;
            let c = c;
            let d = d;
        );
        (a, b, c, d)
    }

    fn try_get_mut2<'a, T: ?Sized, U: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
    ) -> Option<(&'a mut T, &'a mut U)> {
        load_all!(
            self =>
            else return None =>
            let a = a;
            let b = b;
        );
        Some((a, b))
    }

    fn try_get_mut3<'a, T: ?Sized, U: ?Sized, V: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
    ) -> Option<(&'a mut T, &'a mut U, &'a mut V)> {
        load_all!(
            self =>
            else return None =>
            let a = a;
            let b = b;
            let c = c;
        );
        Some((a, b, c))
    }

    fn try_get_mut4<'a, T: ?Sized, U: ?Sized, V: ?Sized, X: ?Sized>(
        &'a mut self,
        a: &'a UtCell<T, Self>,
        b: &'a UtCell<U, Self>,
        c: &'a UtCell<V, Self>,
        d: &'a UtCell<X, Self>,
    ) -> Option<(&'a mut T, &'a mut U, &'a mut V, &'a mut X)> {
        load_all!(
            self =>
            else return None =>
            let a = a;
            let b = b;
            let c = c;
            let d = d;
        );
        Some((a, b, c, d))
    }
}

#[repr(C)]
pub struct UtCell<T: ?Sized, C: CellOwner + ?Sized> {
    token: C::Token,
    value: UnsafeCell<T>,
}

fn validate_trivial_token<T: TrivialToken>(get_align: impl FnOnce() -> usize) {
    fn illegal_trivial_token<T>() -> ! {
        panic!(
            "Token {} is not a valid `TrivialToken`",
            core::any::type_name::<T>()
        )
    }

    // ensure that Self is the same size and align as T
    if mem::size_of::<T>() != 0 || mem::align_of::<T>() != 1 && mem::align_of::<T>() > get_align() {
        illegal_trivial_token::<T>()
    }

    // assert that there is a value of C::Token
    let _value: T = TrivialToken::NEW;
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C>
where
    C::Token: TrivialToken,
{
    #[inline]
    pub fn from_mut(x: &mut T) -> &mut Self {
        validate_trivial_token::<C::Token>(|| mem::align_of_val(x));

        unsafe { &mut *(x as *mut T as *mut Self) }
    }
}

impl<T, C: CellOwner + ?Sized> UtCell<[T], C>
where
    C::Token: TrivialToken,
{
    #[inline]
    pub fn as_slice_of_cells(&self) -> &[UtCell<T, C>] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &*(self as *const Self as *const [UtCell<T, C>]) }
    }

    #[inline]
    pub fn from_slice_of_cells(slice: &[UtCell<T, C>]) -> &Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &*(slice as *const [UtCell<T, C>] as *const Self) }
    }

    #[inline]
    pub fn as_slice_of_cells_mut(&mut self) -> &mut [UtCell<T, C>] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &mut *(self as *mut Self as *mut [UtCell<T, C>]) }
    }

    #[inline]
    pub fn from_slice_of_cells_mut(slice: &mut [UtCell<T, C>]) -> &mut Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &mut *(slice as *mut [UtCell<T, C>] as *mut Self) }
    }
}

impl<T, C: CellOwner + ?Sized, const N: usize> UtCell<[T; N], C>
where
    C::Token: TrivialToken,
{
    #[inline]
    pub fn as_array_of_cells(&self) -> &[UtCell<T, C>; N] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &*(self as *const Self as *const [UtCell<T, C>; N]) }
    }

    #[inline]
    pub fn from_array_of_cells(array: &[UtCell<T, C>; N]) -> &Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &*(array as *const [UtCell<T, C>; N] as *const Self) }
    }

    #[inline]
    pub fn as_array_of_cells_mut(&mut self) -> &mut [UtCell<T, C>; N] {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &mut *(self as *mut Self as *mut [UtCell<T, C>; N]) }
    }

    #[inline]
    pub fn from_array_of_cells_mut(array: &mut [UtCell<T, C>; N]) -> &mut Self {
        validate_trivial_token::<C::Token>(mem::align_of::<T>);

        unsafe { &mut *(array as *mut [UtCell<T, C>; N] as *mut Self) }
    }
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C> {
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<T, C: CellOwner + ?Sized> UtCell<T, C> {
    pub fn new(owner: &C, value: T) -> Self {
        Self::from_token(owner.token(), value)
    }

    pub fn from_token(token: C::Token, value: T) -> Self {
        Self {
            token,
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized, C: CellOwner + ?Sized> UtCell<T, C> {
    pub fn is_owned_by(&self, owner: &C) -> bool {
        owner.owns(&self.token)
    }

    pub fn assert_owned_by(&self, owner: &C) {
        fn assert_owned_by_failed<T: ?Sized>() -> ! {
            panic!(
                "Tried to access a {} with a value that doesn't own the cell",
                core::any::type_name::<T>()
            )
        }

        if !self.is_owned_by(owner) {
            assert_owned_by_failed::<Self>()
        }
    }

    pub fn load<'a>(&'a self, owner: &'a C) -> &'a T {
        self.assert_owned_by(owner);
        unsafe { &*self.as_ptr() }
    }

    pub fn load_mut<'a>(&'a self, owner: &'a mut C) -> &'a mut T {
        self.assert_owned_by(owner);
        unsafe { &mut *self.as_ptr() }
    }

    #[doc(hidden)]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn load_mut_unchecked<'a>(&'a self, _owner: &'a C) -> &'a mut T {
        unsafe { &mut *self.as_ptr() }
    }
}
