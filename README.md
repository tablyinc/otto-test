# Otto: a unified approach to CRDTs and OT

This repo contains tests for `otto`. `otto` enables any boring Rust data structure (so no `Rc`, `RefCell`, etc) to be used as a replicated data type. It supports being used in multiple ways, including:

* Exposing a data structure as an operation-based CRDT by wrapping it with `Crdt<T>`.
* Using OT with a synchronising server, by calling methods `insert_and_rebase_forward`, `insert_and_rebase_back` and `converge`.

It also supports synchronising of clients using both CRDT and OT techniques.

## Data types supported

| Data type | Operations supported |
|---|---|
| `List<T>` | `insert_at`, `delete_at`, `map_at` |
| `Register<T>` | `set`, `map` |
| `(A, B, ...)` | `map_a`, `map_b`, ... |
| `#[derive(State)]` for arbitrary structs and enums | `map_field_a`, `map_field_b`, ... |
| `bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char usize isize` | - |

## License
Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
