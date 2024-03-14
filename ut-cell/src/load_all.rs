use crate::{CellOwner, UtCell};

#[doc(hidden)]
#[macro_export]
macro_rules! __load_all_hlist {
    () => {
        $crate::load_all::Nil
    };
    ($value:expr $(, $rest:expr)* $(,)?) => {
        $crate::load_all::Cons {
            value: match $value {
                ref value => {
                    let value: &$crate::UtCell<_, _> = value;
                    value
                }
            },
            rest: $crate::__load_all_hlist![$($rest),*]
        }
    };
}

#[macro_export]
macro_rules! load_all {
    ($owner:expr => $(let $name:pat = $value:expr;)*) => {
        let hlist = $crate::__load_all_hlist![$($value),*];
        let owner: &mut _ = $owner;

        $crate::load_all::CellList::assert_owned_by(&hlist, owner);
        $crate::load_all::CellList::assert_all_elements_unique(&hlist);

        $(
            let $name = unsafe { hlist.value.load_mut_unchecked(owner) };
            let hlist = hlist.rest;
        )*

        let $crate::load_all::Nil = hlist;
    };
}

pub trait Seal {}

pub unsafe trait CellList: Seal {
    type Owner: CellOwner + ?Sized;

    fn is_owned_by(&self, owner: &Self::Owner) -> bool;

    fn assert_owned_by(&self, owner: &Self::Owner);

    fn contains(&self, ptr: *mut ()) -> bool;

    fn all_elements_unique(&self) -> bool;

    fn assert_all_elements_unique(&self);
}

#[derive(Debug, Clone, Copy)]
pub struct Nil;
#[derive(Debug, Clone, Copy)]
pub struct Cons<T, Ts> {
    pub value: T,
    pub rest: Ts,
}

impl Seal for Nil {}
impl<T, Ts: Seal> Seal for Cons<T, Ts> {}
unsafe impl<'a, T: ?Sized, C: CellOwner + ?Sized> CellList for Cons<&'a UtCell<T, C>, Nil> {
    type Owner = C;

    fn is_owned_by(&self, owner: &Self::Owner) -> bool {
        self.value.is_owned_by(owner)
    }

    fn assert_owned_by(&self, owner: &Self::Owner) {
        self.value.assert_owned_by(owner);
    }

    fn contains(&self, ptr: *mut ()) -> bool {
        // ZSTs don't overlap
        !self.is_head_zst_value() && ptr == self.value.as_ptr().cast()
    }

    fn all_elements_unique(&self) -> bool {
        true
    }

    fn assert_all_elements_unique(&self) {}
}

unsafe impl<'a, T: ?Sized, Ts: CellList> CellList for Cons<&'a UtCell<T, Ts::Owner>, Ts> {
    type Owner = Ts::Owner;

    fn is_owned_by(&self, owner: &Self::Owner) -> bool {
        self.value.is_owned_by(owner) && self.rest.is_owned_by(owner)
    }

    fn assert_owned_by(&self, owner: &Self::Owner) {
        self.value.assert_owned_by(owner);
        self.rest.assert_owned_by(owner)
    }

    fn contains(&self, ptr: *mut ()) -> bool {
        // ZSTs don't overlap
        !self.is_head_zst_value() && ptr == self.value.as_ptr().cast() || self.rest.contains(ptr)
    }

    fn all_elements_unique(&self) -> bool {
        !self.is_head_in_tail() && self.rest.all_elements_unique()
    }

    fn assert_all_elements_unique(&self) {
        assert!(!self.is_head_in_tail());
        self.rest.assert_all_elements_unique();
    }
}

impl<T: ?Sized, O: CellOwner + ?Sized, Ts> Cons<&UtCell<T, O>, Ts> {
    fn is_head_zst_value(&self) -> bool {
        core::mem::size_of_val(&self.value.value) == 0
    }
}

impl<T: ?Sized, Ts: CellList> Cons<&UtCell<T, Ts::Owner>, Ts> {
    fn is_head_in_tail(&self) -> bool {
        // ZSTs don't overlap
        !self.is_head_zst_value() && self.rest.contains(self.value.as_ptr().cast())
    }
}
