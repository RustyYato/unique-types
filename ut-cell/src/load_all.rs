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

        if let $crate::Result::Err(err) = $crate::load_all::CellList::is_owned_by(&hlist, owner, 0) {
            $crate::Result::Err(err)
        } else if let $crate::Result::Err(err) = $crate::load_all::CellList::all_elements_unique(&hlist, 0) {
            $crate::Result::Err(err)
        } else {
            $(
                // SAFETY: CellList asserts that all values are owned by the owner
                // and that all values in the list are unique
                let $value = unsafe { hlist.value.load_mut_unchecked(owner) };
                let hlist = hlist.rest;
            )*

            let $crate::load_all::Nil = hlist;

            $crate::Result::Ok(($($value),*))
        }
    }};
    ($owner:expr => $($value:ident),+ $(,)?) => {{
        $crate::load_all![$owner => try $($value),*].unwrap()
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

    fn is_owned_by(&self, owner: &Self::Owner, i: usize) -> Result<(), super::TryLoadAllError>;

    fn overlaps_with(
        &self,
        ptr: *const u8,
        size: usize,
        a: usize,
        b: usize,
    ) -> Result<(), super::TryLoadAllError>;

    fn all_elements_unique(&self, i: usize) -> Result<(), super::TryLoadAllError>;
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
unsafe impl<T: ?Sized, C: CellOwner + ?Sized> CellList for Cons<&UtCell<T, C>, Nil> {
    type Owner = C;

    fn is_owned_by(&self, owner: &Self::Owner, i: usize) -> Result<(), super::TryLoadAllError> {
        if self.value.is_owned_by(owner) {
            Ok(())
        } else {
            Err(super::TryLoadAllError::NotOwned { arg: i })
        }
    }

    fn overlaps_with(
        &self,
        ptr: *const u8,
        size: usize,
        a: usize,
        b: usize,
    ) -> Result<(), super::TryLoadAllError> {
        if self.head_overlaps_with(ptr, size) {
            Err(super::TryLoadAllError::Overlaps { a, b })
        } else {
            Ok(())
        }
    }

    fn all_elements_unique(&self, _: usize) -> Result<(), super::TryLoadAllError> {
        Ok(())
    }
}
// SAFETY:
//
// assert_owned_by does check that all values in the list are owned by the given owner
// the head is checked that it doesn't overlap with any other element in the list
// overlaps_with does check that all cells in the list don't overlap with the given memory region
unsafe impl<T: ?Sized, Ts: CellList> CellList for Cons<&UtCell<T, Ts::Owner>, Ts> {
    type Owner = Ts::Owner;

    fn is_owned_by(&self, owner: &Self::Owner, i: usize) -> Result<(), crate::TryLoadAllError> {
        if self.value.is_owned_by(owner) {
            self.rest.is_owned_by(owner, i + 1)
        } else {
            Err(crate::TryLoadAllError::NotOwned { arg: i })
        }
    }

    fn overlaps_with(
        &self,
        ptr: *const u8,
        size: usize,
        a: usize,
        b: usize,
    ) -> Result<(), crate::TryLoadAllError> {
        if self.head_overlaps_with(ptr, size) {
            Err(crate::TryLoadAllError::Overlaps { a, b })
        } else {
            self.rest.overlaps_with(ptr, size, a, b + 1)
        }
    }

    fn all_elements_unique(&self, i: usize) -> Result<(), crate::TryLoadAllError> {
        // ZSTs don't overlap
        if self.is_head_zst_value() {
            return Ok(());
        }

        let (this, size) = self.head_range();
        debug_assert!(size != 0);

        self.rest.overlaps_with(this, size, i, i + 1)?;

        self.rest.all_elements_unique(i + 1)
    }
}

impl<T: ?Sized, O: CellOwner + ?Sized, Ts> Cons<&UtCell<T, O>, Ts> {
    const fn is_head_zst_value(&self) -> bool {
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

    const fn head_range(&self) -> (*const u8, usize) {
        (
            self.value as *const UtCell<T, O> as *const u8,
            core::mem::size_of_val::<UtCell<T, O>>(self.value),
        )
    }
}
