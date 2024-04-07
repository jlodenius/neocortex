# neocortex

<div align="center"><img src="img/dr_neo_cortex.png" width="200" height="200"></div>

Shared memory crate designed for simplicity, safety, and extensibility. With minimal dependencies on `libc` and `tracing`, this crate wraps unsafe shared memory operations in a user-friendly API.

## System Requirements

- **Operating System**: Linux, macOS, or other UNIX-like operating systems.
- **Dependencies**: Users must ensure `libc` is available in their system's standard library.

## Safety Guarantees

- **Error Handling**: As `libc` syscalls are inherently unsafe, no guarantees can be made that all allocated resources are properly cleaned up on a failure. This crate provides two error variants, `CleanSystem` and `DirtySystem` to indicate whether or not the error is leaving any dangling resources. All system errors also provides additional error information from the operating system on top of our custom error messages.
- **Error Logging**: As an additional safety guarantee, all `DirtySystem` errors that are not properly handled (currently only in some `Drop` implementations) will emit a `tracing::error!` event.

## Features
- **Simple API**: Offers an easy-to-use interface for shared memory operations, abstracting `libc` complexities.
- **Clear Error Handling**: Distinguishes between `Clean` and `Dirty` system errors.
- **Built-in Synchronization**: Includes a semaphore-based lock for safe shared memory access. *(requires crate feature "semaphore")*.
- **Extendable**: Flexibility to implement custom synchronization logic through the `CortexSync` trait.

Simple example using the built-in semaphore lock:

```rust
// Creating a segment of shared memory
let key = 123;
let data: f64 = 42.0;
let cortex_1: Cortex<_, Semaphore> = Cortex::new(key, data, None).unwrap();
assert_eq!(cortex_1.read().unwrap(), 42.0);

// Attaching to an already existing segment of shared memory requires explicit type annotations
let cortex_2: Cortex<f64, Semaphore> = Cortex::attach(key).unwrap();
assert_eq!(cortex_1.read().unwrap(), cortex_2.read().unwrap());
```

The `semaphore` module comes with some pre-defined permissions, setting it to `None` like the example above will default to `OwnerOnly` which is the most restrictive mode.

```rust
let key = 123;
let data: f64 = 42.0;
let settings = SemaphoreSettings {
    mode: SemaphorePermission::OwnerAndGroup,
};
let cortex: Cortex<_, Semaphore> = Cortex::new(key, data, Some(&settings)).unwrap();
```
