# neocortex

<div align="center"><img src="img/dr_neo_cortex.png" width="200" height="200"></div>

Shared memory crate designed for simplicity, safety, and extensibility. With minimal dependencies on `libc` and `tracing`, this crate wraps unsafe shared memory operations in a user-friendly API.

## Quick Start
Install using the *(currently)* only built-in lock implementation. See examples below for more instructions.
```bash
cargo add neocortex --features semaphore
```

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


## Examples

Simple example using the built-in semaphore lock:

```rust
use neocortex::{Semaphore, SemaphoreSettings, CortexBuilder};

// Initialize a segment of shared memory with the value 42.0
let key = 123;
let cortex = CortexBuilder::new(42.0)
    .key(key)
    .with_default_lock::<Semaphore>()
    .unwrap();

// Attaching to an existing segment of shared memory requires explicit type annotations
let attached: Cortex<f64, Semaphore> = Cortex::attach(key).unwrap();

assert_eq!(cortex.read().unwrap(), attached.read().unwrap());

// Write to shared memory
let new_val = 12.34;
cortex.write(new_val).unwrap();

assert_eq!(cortex.read().unwrap(), new_val);
```

The `semaphore` module comes with some pre-defined permissions, these permissions dictates which OS users can interact with the semaphore. Using `with_default_lock` defaults to `OwnerOnly` which is the most restrictive mode. Check out `SemaphorePermission` for other modes, or use the `Custom` enum-variant to set your own permissions.

```rust
use neocortex::{Semaphore, SemaphoreSettings, SemaphorePermission, CortexBuilder};

let settings = SemaphoreSettings {
    mode: SemaphorePermission::OwnerAndGroup,
};

let cortex = CortexBuilder::new(42.0)
    .key(123)
    .with_lock::<Semaphore>(&settings)
    .unwrap();
```
