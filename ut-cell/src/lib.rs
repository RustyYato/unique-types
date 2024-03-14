#![no_std]

use core::{cell::UnsafeCell, mem};

use unique_types::{TrivialToken, UniqueType};

pub trait CellOwner: UniqueType {}
impl<T: ?Sized + UniqueType> CellOwner for T {}

#[repr(C)]
pub struct UtCell<T: ?Sized, C: CellOwner> {
    token: C::Token,
    value: UnsafeCell<T>,
}

impl<T: ?Sized, C: CellOwner> UtCell<T, C>
where
    C::Token: TrivialToken,
{
    #[inline]
    pub fn from_mut(x: &mut T) -> &mut Self {
        fn illegal_trivial_token<T>() -> ! {
            panic!(
                "Token {} is not a valid `TrivialToken`",
                core::any::type_name::<T>()
            )
        }

        // ensure that Self is the same size and align as T
        if mem::size_of::<C::Token>() != 0
            || mem::align_of::<C::Token>() != 1
                && mem::align_of::<C::Token>() > mem::align_of_val::<T>(x)
        {
            illegal_trivial_token::<C::Token>()
        }
        // assert that there is a value of C::Token
        let _value: C::Token = TrivialToken::NEW;

        unsafe { &mut *(x as *mut T as *mut Self) }
    }
}

impl<T: ?Sized, C: CellOwner> UtCell<T, C> {
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<T: ?Sized, C: CellOwner> UtCell<T, C> {
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
}
