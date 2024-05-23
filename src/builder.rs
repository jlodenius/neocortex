use crate::{Cortex, CortexResult, CortexSync};
use std::marker::PhantomData;

pub trait BuilderState {}

pub struct Uninitialized {}
pub struct Initialized {}
pub struct WithKey {}

impl BuilderState for Uninitialized {}
impl BuilderState for Initialized {}
impl BuilderState for WithKey {}

pub struct CortexBuilder<T, S> {
    data: T,
    key: Option<i32>,
    state: PhantomData<S>,
}

impl<T> CortexBuilder<T, Uninitialized> {
    pub fn new(data: T) -> CortexBuilder<T, Initialized> {
        CortexBuilder {
            data,
            key: None,
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
            state: PhantomData,
        }
    }
    /// Attempt to generate a random key
    pub fn random_key(self) -> CortexBuilder<T, WithKey> {
        CortexBuilder {
            data: self.data,
            key: None,
            state: PhantomData,
        }
    }
}

impl<T> CortexBuilder<T, WithKey> {
    /// Attempts to construct a `Cortex` with custom lock settings that will differ depending on
    /// your lock implementation
    pub fn with_lock<L: CortexSync>(
        self,
        lock_settings: &L::Settings,
    ) -> CortexResult<Cortex<T, L>> {
        Cortex::new(self.key, self.data, Some(lock_settings))
    }
    /// Attempts to construct a `Cortex` without passing any lock settings
    pub fn with_default_lock<L: CortexSync>(self) -> CortexResult<Cortex<T, L>> {
        Cortex::new(self.key, self.data, None)
    }
}
