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
    ($owner:expr => try $($value:ident),+ $(,)?) => {{
        let hlist = $crate::__load_all_hlist![$($value),*];
        let owner: &mut _ = $owner;

        $crate::load_all::CellList::assert_owned_by(&hlist, owner);
        if $crate::load_all::CellList::all_elements_unique(&hlist) {
            $(
                // SAFETY: CellList asserts that all values are owned by the owner
                // and that all values in the list are unique
                let $value = unsafe { hlist.value.load_mut_unchecked(owner) };
                let hlist = hlist.rest;
            )*

            let $crate::load_all::Nil = hlist;

            Some(($($value),*))
        } else {
            None
        }
    }};
    ($owner:expr => $($value:ident),+ $(,)?) => {{
        let hlist = $crate::__load_all_hlist![$($value),*];
        let owner: &mut _ = $owner;

        $crate::load_all::CellList::assert_owned_by(&hlist, owner);
        $crate::load_all::CellList::assert_all_elements_unique(&hlist);

        $(
            // SAFETY: CellList asserts that all values are owned by the owner
            // and that all values in the list are unique
            let $value = unsafe { hlist.value.load_mut_unchecked(owner) };
            let hlist = hlist.rest;
        )*

        let $crate::load_all::Nil = hlist;

        ($($value),*)
    }};
}

pub trait Seal {}

/// # Safety
///
/// assert_owned_by must check that all values in the list are owned by the given owner
/// assert_all_elements_unique must check that all values in the list do no overlap
/// overlaps_with must check that all cells in the list don't overlap with the given memory region
pub unsafe trait CellList: Seal {
    type Owner: CellOwner + ?Sized;

    fn assert_owned_by(&self, owner: &Self::Owner);

    fn overlaps_with(&self, ptr: *const u8, size: usize) -> bool;

    fn all_elements_unique(&self) -> bool;

    fn assert_all_elements_unique(&self) {
        #[cold]
        #[inline(never)]
        fn assert_failed() -> ! {
            panic!("Detected overlapping cells while trying to load_all");
        }

        if !self.all_elements_unique() {
            assert_failed()
        }
    }
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
// SAFETY:
//
// assert_owned_by does check that all values in the list are owned by the given owner
// there is only one element in the list, so there can't be any overlaps
// overlaps_with does check that all cells in the list don't overlap with the given memory region
unsafe impl<'a, T: ?Sized, C: CellOwner + ?Sized> CellList for Cons<&'a UtCell<T, C>, Nil> {
    type Owner = C;

    fn assert_owned_by(&self, owner: &Self::Owner) {
        self.value.assert_owned_by(owner);
    }

    fn overlaps_with(&self, ptr: *const u8, size: usize) -> bool {
        // ZSTs don't overlap
        self.head_overlaps_with(ptr, size)
    }

    fn all_elements_unique(&self) -> bool {
        true
    }
}
// SAFETY:
//
// assert_owned_by does check that all values in the list are owned by the given owner
// the head is checked that it doesn't overlap with any other element in the list
// overlaps_with does check that all cells in the list don't overlap with the given memory region
unsafe impl<'a, T: ?Sized, Ts: CellList> CellList for Cons<&'a UtCell<T, Ts::Owner>, Ts> {
    type Owner = Ts::Owner;

    fn assert_owned_by(&self, owner: &Self::Owner) {
        self.value.assert_owned_by(owner);
        self.rest.assert_owned_by(owner)
    }

    fn overlaps_with(&self, ptr: *const u8, size: usize) -> bool {
        // ZSTs don't overlap
        self.head_overlaps_with(ptr, size) || self.rest.overlaps_with(ptr, size)
    }

    fn all_elements_unique(&self) -> bool {
        !self.is_head_in_tail() && self.rest.all_elements_unique()
    }
}

impl<T: ?Sized, O: CellOwner + ?Sized, Ts> Cons<&UtCell<T, O>, Ts> {
    fn is_head_zst_value(&self) -> bool {
        core::mem::size_of_val(&self.value.value) == 0
    }

    fn head_overlaps_with(&self, ptr: *const u8, size: usize) -> bool {
        if self.is_head_zst_value() {
            return false;
        }

        let (this, this_size) = self.head_range();
        debug_assert!(this_size != 0);
        debug_assert!(size != 0);

        if core::mem::size_of::<O::Token>() == 0 {
            // if the token is a ZST, then it's possible for the cells to overlap
            // so we need to do a full range overlap check
            let this_end = this.wrapping_add(this_size);
            let end = ptr.wrapping_add(size);

            this < end && ptr < this_end
        } else {
            // if the token is not a ZST, then it is impossible for cells to overlap
            // in this case just use pointer identity since that will be correct
            ptr == this
        }
    }

    fn head_range(&self) -> (*const u8, usize) {
        (
            self.value as *const UtCell<T, O> as *const u8,
            core::mem::size_of_val::<UtCell<T, O>>(self.value),
        )
    }
}

impl<T: ?Sized, Ts: CellList> Cons<&UtCell<T, Ts::Owner>, Ts> {
    fn is_head_in_tail(&self) -> bool {
        // ZSTs don't overlap
        if self.is_head_zst_value() {
            return false;
        }

        let (this, size) = self.head_range();
        debug_assert!(size != 0);

        self.rest.overlaps_with(this, size)
    }
}
