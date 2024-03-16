use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng, SeedableRng,
};
use ut_arena::{
    generation::g8, generic_dense::GenericDenseArena, generic_sparse::GenericSparseArena,
};

type ArenaKey = ut_arena::key::ArenaKey<usize, g8>;

trait Arena: IndexMut<ArenaKey, Output = char> {
    fn new() -> Self;

    fn insert(&mut self, value: char) -> ArenaKey;

    fn remove(&mut self, key: ArenaKey) -> char;

    fn get(&self, key: ArenaKey) -> Option<&char>;

    fn get_mut(&mut self, key: ArenaKey) -> Option<&mut char>;

    fn try_remove(&mut self, key: ArenaKey) -> Option<char>;
}

impl Arena for GenericDenseArena<char, (), g8> {
    fn new() -> Self {
        Self::new()
    }

    fn insert(&mut self, value: char) -> ArenaKey {
        self.insert(value)
    }

    fn remove(&mut self, key: ArenaKey) -> char {
        self.remove(key)
    }

    fn get(&self, key: ArenaKey) -> Option<&char> {
        self.get(key)
    }

    fn get_mut(&mut self, key: ArenaKey) -> Option<&mut char> {
        self.get_mut(key)
    }

    fn try_remove(&mut self, key: ArenaKey) -> Option<char> {
        self.try_remove(key)
    }
}

impl Arena for GenericSparseArena<char, (), g8> {
    fn new() -> Self {
        Self::new()
    }

    fn insert(&mut self, value: char) -> ArenaKey {
        self.insert(value)
    }

    fn remove(&mut self, key: ArenaKey) -> char {
        self.remove(key)
    }

    fn get(&self, key: ArenaKey) -> Option<&char> {
        self.get(key)
    }

    fn get_mut(&mut self, key: ArenaKey) -> Option<&mut char> {
        self.get_mut(key)
    }

    fn try_remove(&mut self, key: ArenaKey) -> Option<char> {
        self.try_remove(key)
    }
}

fn test_arena<A: Arena>() {
    let mut arena = A::new();
    let mut map = HashMap::new();
    let mut dead_keys = Vec::new();

    let seed = if true { rand::random() } else { unreachable!() };
    let mut rng = StdRng::from_seed(seed);

    scopeguard::defer_on_unwind! {
        println!("SEED: {seed:?}");
    }

    for _ in 0..1024 * 64 {
        match rng.gen_range(0..=4) {
            0 => {
                let x = rng.gen();
                let key = arena.insert(x);
                assert!(!map.contains_key(&key));
                map.insert(key, x);
            }
            1 => {
                let Some((&key, &val)) = map.iter().choose(&mut rng) else {
                    continue;
                };

                assert_eq!(arena[key], val);
            }
            2 => {
                let Some((&key, val)) = map.iter_mut().choose(&mut rng) else {
                    continue;
                };

                assert_eq!(arena[key], *val);
                *val = rng.gen();
                arena[key] = *val;
            }
            3 => {
                let Some((&key, &val)) = map.iter().choose(&mut rng) else {
                    continue;
                };
                map.remove(&key);

                assert_eq!(arena.remove(key), val);
                dead_keys.push(key);
            }
            4 => {
                let Some(&key) = dead_keys.as_slice().choose(&mut rng) else {
                    continue;
                };

                assert!(arena.get(key).is_none());
                assert!(arena.get_mut(key).is_none());
                assert!(arena.try_remove(key).is_none());
            }
            _ => unreachable!(),
        }
    }

    for key in dead_keys {
        assert!(arena.get(key).is_none());
        assert!(arena.get_mut(key).is_none());
        assert!(arena.try_remove(key).is_none());
    }
}

#[test]
fn test_sparse_arena() {
    test_arena::<GenericSparseArena<_, _, _, _>>();
}

#[test]
fn test_dense_arena() {
    test_arena::<GenericDenseArena<_, _, _, _>>();
}
