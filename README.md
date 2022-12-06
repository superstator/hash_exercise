# MiniHash

MiniHash/MiniMap is a quick exercise to build a simple in-memory cache, ala Redis. Goals for the exercise
included basic CRUD methods, optional record expiration, performance verification up to 10m records, and 
thread safety.

## Build/Test

The project may be built with a simple `cargo build` command from the project root, and likewise tests may be run
with `cargo test`. Even when targets are being met, generating a large dataset for a performance test may take several 
minutes, so those tests are segregated into a `perf_test` feature; use `cargo test --features=perf_test` to include them.

## Approaches

Two approaches were considered. In a real production use case, `std::collections::HashMap` plus `Arc<Mutex<>>` 
or async channels (e.g. `Tokio`) could provide a solid threadsafe implementation, or likely an existing library
could be found that already implements a fast, thread-safe hash map. For the purposes of this exercise, I decided
to build out a minimal hashmap myself, with a configurable map size to allow for experimentation, and using 
`Arc<Mutex<>>` to mediate multithreaded usage.

## Results

The basic hashmap implementation was reasonably easy to achieve. First iterations used a fixed-size array for the
internal map, but this made initialization somewhat klunky and required the use of const generics; switching to `Vec`
simplified things without any significant performance penalty, esp. since the map was heap allocated either way. The
`Arc<Mutex<>>` pattern provided simple thread safety with minimal overhead.

Performance goals were reasonably easy to meet; 10m records can be inserted in a map with 100,000 segments or buckets
at a rate of ~83/ms, and then accessed randomly with worst case access times well under 1ms for hits and misses.

Memory tradeoff is clear; a large map is much faster, but consumes more memory even for empty sets. Any operation on
a key requires hashing that key, looking up the corresponding bucket, then scanning the bucket for matching key names,
thus worst case operations will always require (Key count / Map size) reads.

## Warts

This was meant to be a quick exercise, so there are plenty of nits

- Most const generic parameters could be standard parameters, now that the internal map is a simple `Vec`
- Expiration is functional but a bit kludgy; a `get` will not return an expired item, but a `delete` will, for example
- The caller must make an intelligent choice for the map size; a real implementation should be able to select and resize the map itself, and possibly do more on-demand allocation of buckets
- Perf verification just looks at worst case `get()` times, and does not try to quantify other aspects
- etc etc etc