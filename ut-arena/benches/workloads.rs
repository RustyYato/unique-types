use std::hint::black_box;

use criterion::{criterion_group, Criterion};
use rand::Rng;
use ut_arena::{dense_slab::DenseSlab, generation::NoGeneration, slab::Slab};

#[derive(Debug, Clone, Copy)]
enum Action {
    Insert(char),
    Remove(usize),
    Access(usize),
}

#[derive(Debug, Clone, Copy)]
enum ActionType {
    Insert,
    Remove,
    Access,
}

#[derive(Clone, Copy)]
struct WorkloadConfig {
    inserts: usize,
    removals: usize,
    accesses: usize,
}

fn make_workload(rng: &mut impl Rng, config: WorkloadConfig) -> Vec<Action> {
    let mut workload = Vec::new();
    assert!(config.removals <= config.inserts);

    let mut pool = Vec::new();
    pool.extend(std::iter::repeat_n(ActionType::Insert, config.inserts));
    pool.extend(std::iter::repeat_n(ActionType::Remove, config.removals));
    pool.extend(std::iter::repeat_n(ActionType::Access, config.accesses));
    let mut pool_removed = Vec::new();
    let mut may_access = Vec::new();

    'shuffle: loop {
        let mut slab = Slab::<char>::new();
        workload.clear();
        may_access.clear();
        pool.append(&mut pool_removed);
        let mut inserts_left = config.inserts;

        while !pool.is_empty() {
            let i = rng.random_range(0..pool.len());

            match pool[i] {
                ActionType::Insert => {
                    inserts_left -= 1;
                    let c = rng.random();
                    let key = slab.insert(c);
                    may_access.push(key);
                    pool_removed.push(pool.remove(i));
                    workload.push(Action::Insert(c));
                }
                ActionType::Remove => {
                    if may_access.is_empty() {
                        continue;
                    }

                    let x = rng.random_range(0..may_access.len());
                    let key = may_access.remove(x);
                    slab.remove(key);
                    workload.push(Action::Remove(key));
                    pool_removed.push(pool.remove(i));
                }
                ActionType::Access => {
                    if may_access.is_empty() {
                        if inserts_left == 0 {
                            continue 'shuffle;
                        }

                        continue;
                    }

                    let x = rng.random_range(0..may_access.len());
                    let key = may_access[x];
                    workload.push(Action::Access(key));
                    pool_removed.push(pool.remove(i));
                }
            }
        }
        break;
    }

    let actions = config.inserts + config.removals + config.accesses;

    assert_eq!(pool_removed.len(), actions);
    assert_eq!(workload.len(), actions);

    workload
}

fn run_sparse(c: &mut Criterion) {
    let mut bench_workload = move |name: &str, config: WorkloadConfig| {
        let workload = make_workload(&mut rand::rng(), config);

        c.benchmark_group(name)
            .throughput(criterion::Throughput::Elements(workload.len() as u64))
            .bench_function("slab", |b| {
                b.iter(|| run_workload_slab(&workload));
            })
            .bench_function("sparse", |b| {
                b.iter(|| run_workload_sparse(&workload));
            })
            .bench_function("sparse-lt", |b| {
                b.iter(|| run_workload_sparse_lt(&workload));
            })
            .bench_function("dense", |b| {
                b.iter(|| run_workload_dense(&workload));
            });
    };

    bench_workload(
        "insert-removal",
        WorkloadConfig {
            inserts: 1024,
            removals: 1024,
            accesses: 0,
        },
    );

    bench_workload(
        "insert-heavy",
        WorkloadConfig {
            inserts: 1024,
            removals: 64,
            accesses: 64,
        },
    );

    bench_workload(
        "read-heavy-small",
        WorkloadConfig {
            inserts: 64,
            removals: 64,
            accesses: 1024,
        },
    );

    bench_workload(
        "read-heavy-large",
        WorkloadConfig {
            inserts: 1024,
            removals: 1024,
            accesses: 1024,
        },
    );
}

fn run_workload_sparse(workload: &[Action]) {
    let mut slab = Slab::new();
    for &action in workload {
        match action {
            Action::Insert(c) => {
                slab.insert(c);
            }
            Action::Remove(key) => {
                slab.remove(key);
            }
            Action::Access(key) => {
                black_box(slab[key]);
            }
        }
    }
}

fn run_workload_dense(workload: &[Action]) {
    let mut slab = DenseSlab::new();
    for &action in workload {
        match action {
            Action::Insert(c) => {
                slab.insert(c);
            }
            Action::Remove(key) => {
                slab.remove(key);
            }
            Action::Access(key) => {
                black_box(slab[key]);
            }
        }
    }
}

fn run_workload_slab(workload: &[Action]) {
    let mut slab = slab::Slab::new();
    for &action in workload {
        match action {
            Action::Insert(c) => {
                slab.insert(c);
            }
            Action::Remove(key) => {
                slab.remove(key);
            }
            Action::Access(key) => {
                black_box(slab[key]);
            }
        }
    }
}

fn run_workload_sparse_lt(workload: &[Action]) {
    unique_types::unique_lifetime!(lt);
    let mut slab =
        ut_arena::generic_sparse::GenericSparseArena::<_, _, NoGeneration>::with_owner(lt);
    for &action in workload {
        match action {
            Action::Insert(c) => {
                slab.insert::<usize>(c);
            }
            Action::Remove(key) => {
                let key = unsafe { ut_vec::UtIndex::new_unchecked(key, slab.owner()) };
                slab.remove(key);
            }
            Action::Access(key) => {
                let key = unsafe { ut_vec::UtIndex::new_unchecked(key, slab.owner()) };
                black_box(slab[key]);
            }
        }
    }
}

criterion_group! {
    bench_workloads, run_sparse
}

criterion::criterion_main! { bench_workloads }
