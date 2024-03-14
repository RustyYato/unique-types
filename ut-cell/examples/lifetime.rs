use ut_cell::{CellOwner, UtCell};

use unique_types::lifetime::{LifetimeUt, LifetimeUtToken};

fn main() {
    LifetimeUt::with(|mut ut| {
        let cell_a = UtCell::from_token(LifetimeUtToken::new(), 30);
        assert_eq!(*cell_a.load(&ut), 30);
        assert_eq!(*cell_a.load_mut(&mut ut), 30);
        *cell_a.load_mut(&mut ut) = 20;
        assert_eq!(*cell_a.load(&ut), 20);
        assert_eq!(*cell_a.load_mut(&mut ut), 20);

        let cell_b = UtCell::from_token(LifetimeUtToken::new(), 50);
        let (a, b) = ut.get_mut2(&cell_a, &cell_b);
        assert_eq!(*a, 20);
        assert_eq!(*b, 50);

        *a = 0;
        *b = 20;

        assert_eq!(*cell_a.load(&ut), 0);
        assert_eq!(*cell_b.load(&ut), 20);

        let mut cell_c_value = 0;
        let cell_c = UtCell::from_mut(&mut cell_c_value);
        assert_eq!(*cell_c.load(&ut), 0);
    });
}
