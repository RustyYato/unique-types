## Unique Types

Unique types build on the ideas from [`qcell`](https://crates.io/crates/qcell), generalizing and applying them to other contexts. The core abstractions live in `unique-types`, and this is used in...
* `ut-cell` - which is equivalent to `qcell`
* `ut-vec` - a grow only vector which allows for compile-time checked indexing
* `ut-arena` - which builds on `ut-vec` and provides compile time checked indexing for arenas and extremely customizable arenas. This generalizes over [`slotmap`](https://crates.io/crates/slotmap)  and [`slab`](https://crates.io/crates/slab)

## Unique Types

The core of these libraries is the `UniqueType` trait, an unsafe trait which specifies that every value of a type is *unique* in some way. For example, we could attach a runtime id for every value which ensures that every value is unique. Or we could use some lifetime tricks from [ghost-cell](https://crates.io/crates/ghost-cell) to ensure that the type is unique at *compile time*.

There are a two axes for runtime `UniqueType`s:
- one-shot/reusable
- inter-thread/thread-local
- id size (`u8`, `u16`, `isize`, etc.)

So, for example, we could have a reusable inter-thread `UniqueType` (see this [example](https://github.com/RustyYato/unique-types/blob/main/unique-types/examples/reuse.rs)). 

You can also create your own runtime `UniqueType` to help distinguish it from other `UniqueType`s. For example, the equivalent of `qcell`'s `TCell` is done like this:

```rust
unique_types::custom_counter! {
    struct MyType;
}

let unique = RuntimeUt::<MyType>::with_counter();
// or
let unique = ReuseRuntimeUt::<MyType>::with_counter();
```
Like `TCell`, `RuntimeUt<MyType>` has a size of 0 and an align of 1, so it is basically just a compile time token. 
The difference here is that instead of using a global registry to ensure that you don't create two `RuntimeUt<MyType>` (like in `qcell` does with `TCell`), creating this `RuntimeUt<MyType>` does a single CAS operation on an atomic boolean. This makes it significantly cheaper to acquire one, even if it takes a bit more setup.

This custom counter macro is extremely flexible, and allows you to create your own runtime counters.
```rust
use core::num::NonZero;
use unique_types::reuse::BoundedVec;

unique_types::custom_counter! {
    // every id is represented as a non-zero u32
    // and will cache up to 16 ids to be reused
    // if this type is used in `ReuseRuntimeUt`
    struct MyType(NonZero<u32>);
    with_reuse Mutex<BoundedVec<NonZero<u32>, 16>>
}

let unique = RuntimeUt::<MyType>::with_counter();
// or
let unique = ReuseRuntimeUt::<MyType>::with_counter();
```

Note that runtime ids in `ReuseRuntimeUt` reuse the last value that was created. This means that if the uses of `ReuseRuntimeUt` are well-nested, then all the ids will be reused.

This works, because the old ids are no longer accessible by the time they are reused.
```rust
let a = ReuseRuntimeUt::<MyType>::with_counter();
let b = ReuseRuntimeUt::<MyType>::with_counter();
drop(b);
drop(a);

let c = ReuseRuntimeUt::<MyType>::with_counter();
let d = ReuseRuntimeUt::<MyType>::with_counter();
// `c` has the same id as `a`
// `d` has the same id as `b`
```
And due to how ids are reused, if the lifetimes `ReuseRuntimeUt` usage is well nested, then the ids will be perfectly reused indefinitely. 
