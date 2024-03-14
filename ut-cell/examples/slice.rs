use unique_types::runtime::RuntimeUt;

use ut_cell::{CellOwner, UtCell};

unique_types::custom_counter! {
    struct Example;
}

fn main() {
    let mut ty = RuntimeUt::<Example>::with_counter();

    let mut data: [u8; 4] = [0, 1, 2, 3];
    let data = UtCell::from_mut(&mut data[..]);

    let slice = data.as_slice_of_cells();
    let a = &slice[..2];
    let b = &slice[1..3];

    // verify that a and b overlap
    assert!(core::ptr::eq(&a[1], &b[0]));

    let a = UtCell::from_slice_of_cells(a);
    let b = UtCell::from_slice_of_cells(b);

    // the two slices overlap, so you can't get mutable access to both of them
    assert!(ty.try_get_mut2(a, b).is_err());

    // this will panic because a and b overlap
    ty.get_mut2(a, b);
}
