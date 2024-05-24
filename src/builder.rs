use crate::{Cortex, CortexResult, CortexSync};
use std::marker::PhantomData;

pub trait BuilderState {}

pub struct Uninitialized {}
pub struct Initialized {}
pub struct WithKey {}
pub struct WithRandomKey {}

impl BuilderState for Uninitialized {}
impl BuilderState for Initialized {}
impl BuilderState for WithKey {}
impl BuilderState for WithRandomKey {}

pub struct CortexBuilder<T, S> {
    data: T,
    force_ownership: bool,
    key: Option<i32>,
    state: PhantomData<S>,
}

impl<T> CortexBuilder<T, Uninitialized> {
    pub fn new(data: T) -> CortexBuilder<T, Initialized> {
        CortexBuilder {
            data,
            key: None,
            force_ownership: false,
            state: PhantomData,
        }
    }
}

impl<T> CortexBuilder<T, Initialized> {
    /// Set a custom key
    pub fn key(self, key: i32) -> CortexBuilder<T, WithKey> {
        CortexBuilder {
            data: self.data,
            key: Some(key),
            force_ownership: self.force_ownership,
            state: PhantomData,
        }
    }
    /// Attempt to generate a random key
    pub fn random_key(self) -> CortexBuilder<T, WithRandomKey> {
        CortexBuilder {
            data: self.data,
            key: None,
            force_ownership: self.force_ownership,
            state: PhantomData,
        }
    }
}

impl<T> CortexBuilder<T, WithKey> {
    ///
    /// Sets the `force_ownership` flag to `true`. If an already existing segment of shared memory
    /// should exist on the selected `key`, with this flag, instead of throwing an error, attempts
    /// to attach to that segment and set `is_owner` to `true`. Meaning this `Cortex` instance will
    /// be responsible for any cleanup.
    ///
    /// # Safety
    ///
    /// Only use this option when you can guarantee that any pre-existing segment of shared memory on
    /// the same `key` is also of the same type `T`.
    ///
    pub fn force_ownership(self) -> CortexBuilder<T, WithKey> {
        CortexBuilder {
            data: self.data,
            key: self.key,
            force_ownership: true,
            state: PhantomData,
        }
    }
}

pub trait KeyState {}
impl KeyState for WithKey {}
impl KeyState for WithRandomKey {}

impl<T, S: KeyState> CortexBuilder<T, S> {
    /// Attempt to construct a `Cortex` with custom lock settings that will differ depending on
    /// your lock implementation
    pub fn with_lock<L: CortexSync>(
        self,
        lock_settings: &L::Settings,
    ) -> CortexResult<Cortex<T, L>> {
        Cortex::new(
            self.key,
            self.data,
            self.force_ownership,
            Some(lock_settings),
        )
    }
    /// Attempt to construct a `Cortex` without passing any lock settings
    pub fn with_default_lock<L: CortexSync>(self) -> CortexResult<Cortex<T, L>> {
        Cortex::new(self.key, self.data, self.force_ownership, None)
    }
}
