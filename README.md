# Otto: a unified approach to CRDTs and OT

This repo contains tests for `otto`. `otto` enables any boring Rust data structure (without `Rc`, `RefCell` etc.) to be used as a replicated data type. It supports achieving convergence via multiple approaches, including:

* Wrapping with `Crdt<T>` to leverage [operation-based CRDT techniques](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type#Operation-based_CRDTs).
* Methods `insert_and_rebase_forward`, `insert_and_rebase_back` and `converge` to leverage [OT techniques](https://en.wikipedia.org/wiki/Operational_transformation) with a synchronising server.

It also supports combining these approaches to synchronise a mix of clients using both CRDT and OT techniques.

## Project aims

* [ ] Minimal bookkeeping (no timestamps, IDs or similar)
* [ ] Rich set of data types (e.g. [`RichText`](https://www.inkandswitch.com/peritext/)) and operations (e.g. `sort` and `group_by` operations on `List<T>`)
* [x] Composability (support arbitrary nesting of types, e.g. `List<(u64, List<u8>)>`)
* [x] Differential dataflow support
* [x] Performance, sufficient for overhead to be negligible in real-world use (within \~1 OoM of [Diamond types](https://josephg.com/blog/crdts-go-brrr/))
* [x] Achieve the strongest known useful properties (e.g. convergence and inverse [properties](https://en.wikipedia.org/wiki/Operational_transformation#Transformation_properties))

## Data types supported

| Data type                                                               | Operations supported |
|-------------------------------------------------------------------------|---|
| `Map<K, V>`                                                             | `insert(K, V)`, `delete(K)`, `map_at(K, V::Instr)`|
| `Set<T>`                                                                | `insert(T)`, `delete(T)`|
| `List<T>`                                                               | `insert_at(usize, T)`, `delete_at(usize)`, `map_at(usize, T::Instr)` |
| `Register<T>`                                                           | `set(T)`, `map(T::Instr)` |
| `(A, B, ...)`                                                           | `map_a(A::Instr)`, `map_b(B::Instr)`, ... |
| `#[derive(State)]` for arbitrary structs and enums                      | `map_field_a(A::Instr)`, `map_field_b(B::Instr)`, ... |
| `bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char usize isize` | - |

## License
Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
