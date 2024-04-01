# neocortex

<div align="center"><img src="img/dr_neo_cortex.png" width="200" height="200"></div>

Shared memory crate designed for simplicity, safety, and extensibility. With minimal dependencies on `libc` and `tracing`, this crate wraps unsafe shared memory operations in a user-friendly API.

## Features:
- **Simple API**: Offers an easy-to-use interface for shared memory operations, abstracting `libc` complexities.
- **Clear Error Handling**: Distinguishes between `Clean` and `Dirty` system errors.
- **Built-in Synchronization**: Includes a semaphore-based lock for safe shared memory access.
- **Extendable**: Flexibility to implement custom synchronization logic through the `CortexSync` trait.

Simple example using the built-in semaphore lock.

```rust
// Creating a segment of shared memory
let key = 123;
let data: f64 = 42.0;
let cortex_1: Cortex<_, Semaphore> = Cortex::new(key, data, None).unwrap();
assert_eq!(cortex_1.read(), 42.0);

// Attaching to an existing segment of shared memory (using an existing key)
let cortex_2: Cortex<f64, Semaphore> = Cortex::attach(key).unwrap();
assert_eq!(cortex_1.read(), cortex_2.read());
```


The `semaphore` module comes with some pre-defined permissions, setting it to `None` like the example above will default to `OwnerOnly` which is the most restrictive mode.

```rust
let key = 123;
let data: f64 = 42.0;
let settings = SemaphoreSettings {
    mode: SemaphorePermission::OwnerAndGroup,
};
let cortex: Cortex<_, Semaphore> = Cortex::new(key, data, Some(settings)).unwrap();
```
