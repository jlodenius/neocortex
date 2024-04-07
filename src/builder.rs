use crate::CortexSync;

trait BuilderState {}

// Different states
struct Initial {}
struct KeySet {}

impl BuilderState for Initial {}
impl BuilderState for KeySet {}

pub struct CortexBuilder<T, L, S> {
    data: T,
    key: i32,
    lock: Option<L>,
    state: std::marker::PhantomData<S>,
}

impl<T, L: CortexSync> CortexBuilder<T, L, Initial> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            key: 1, // todo: random
            lock: None,
            state: std::marker::PhantomData::<Initial>,
        }
    }
    pub fn key(mut self, key: i32) -> Self {
        self.key = key;
        self
    }
}

impl<T, L: CortexSync> CortexBuilder<T, L, KeySet> {
    pub fn lock(mut self, lock_settings: L::Settings) -> Self {
        let lock = L::new(self.key, Some(&lock_settings)).unwrap();
        self.lock.replace(lock);
        self
    }
}

#[test]
fn name() {
    let builder = CortexBuilder::new(123);
}
