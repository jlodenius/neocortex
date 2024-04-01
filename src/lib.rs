mod hive_error;
mod semaphore;

use hive_error::HiveError;
use libc;
use std::fmt::Display;
use tracing;

pub type HiveResult<T> = std::result::Result<T, HiveError>;

/// Attempt to clean up a segment of shared memory
fn try_clear_mem(id: i32) -> HiveResult<()> {
    unsafe {
        if libc::shmctl(id, libc::IPC_RMID, std::ptr::null_mut()) == -1 {
            return Err(HiveError::new_dirty(format!(
                "Error cleaning up shared memory with id: {}",
                id
            )));
        }
    }
    Ok(())
}

pub trait HiveSync: Sized {
    fn new(shmem_key: i32) -> HiveResult<Self>;
    fn attach(shmem_key: i32) -> HiveResult<Self>;
    fn read_lock(&self);
    fn write_lock(&self);
    fn release(&self);
}

pub struct Hive<T, L> {
    key: i32,
    id: i32,
    size: usize,
    is_owner: bool,
    lock: L,
    ptr: *mut T,
}

impl<T, L> Display for Hive<T, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "key: {}, id: {}, size: {}, is_owner: {}",
            self.key, self.id, self.size, self.is_owner
        )
    }
}

impl<T, L: HiveSync> Hive<T, L> {
    /// Allocate a new segment of shared memory
    pub fn new(key: i32, data: T) -> HiveResult<Self> {
        let lock = L::new(key)?;

        // Allocate memory
        let size = std::mem::size_of::<T>();
        let permissions = libc::IPC_CREAT | libc::IPC_EXCL | 0o666;
        let id = unsafe { libc::shmget(key, size, permissions) };
        if id == -1 {
            try_clear_mem(id)?
        } else {
            tracing::trace!("Allocated {} bytes with id {}", size, id);
        }

        // Attach mem to current process and get a pointer
        let ptr = unsafe { libc::shmat(id, std::ptr::null_mut(), 0) as *mut T };
        if ptr as isize == -1 {
            try_clear_mem(id)?;
        } else {
            tracing::trace!("Successfully attached shared memory");
        }

        unsafe {
            ptr.write(data);
        }

        Ok(Self {
            id,
            key,
            size,
            is_owner: true,
            lock,
            ptr,
        })
    }
    /// Attempt to attach to an already existing segment of shared memory
    pub fn attach(key: i32) -> HiveResult<Self> {
        let lock = L::attach(key)?;

        let id = unsafe {
            libc::shmget(key, 0, 0o666) // Size is 0 since we're not creating the segment
        };
        if id == -1 {
            return Err(HiveError::new_clean(format!(
                "Error during shmget for key {}",
                key,
            )));
        } else {
            tracing::trace!("Found shared memory with id {}", id);
        }

        let ptr = unsafe { libc::shmat(id, std::ptr::null_mut(), 0) as *mut T };
        if ptr as isize == -1 {
            return Err(HiveError::new_clean("Error during shmat"));
        } else {
            tracing::trace!("Successfully attached shared memory");
        }

        Ok(Self {
            id,
            key,
            size: std::mem::size_of::<T>(),
            is_owner: false,
            lock,
            ptr,
        })
    }
    /// Read from shared mem
    pub fn read(&self) -> T {
        unsafe {
            self.lock.read_lock();
            let data = self.ptr.read();
            self.lock.release();
            data
        }
    }
    /// Write data to shared mem
    pub fn write(&self, data: T) {
        unsafe {
            self.lock.write_lock();
            self.ptr.write(data);
            self.lock.release();
        }
    }
}

/// Drop a segment of shared memory and clean up its semaphore
impl<T, L> Drop for Hive<T, L> {
    fn drop(&mut self) {
        if !self.is_owner {
            return;
        }
        if let Err(err) = try_clear_mem(self.id) {
            tracing::error!("{err}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semaphore::Semaphore;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn create_shared_mem() {
        use crate::*;

        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let hive: Hive<_, Semaphore> = Hive::new(key, data).unwrap();
        assert_eq!(hive.read(), 42.0);
    }

    #[test]
    fn attach_to_shared_mem() {
        use crate::*;

        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let hive: Hive<_, Semaphore> = Hive::new(key, data).unwrap();
        assert_eq!(hive.read(), 42.0);

        let hive2: Hive<_, Semaphore> = Hive::attach(key).unwrap();
        assert_eq!(hive.read(), hive2.read());
    }

    #[test]
    fn multi_thread() {
        let key = rand::random::<i32>().abs();
        let initial_data: i32 = 42;

        // Create a new shared memory segment
        let _hive: Hive<_, Semaphore> =
            Hive::new(key, initial_data).expect("Failed to create shared memory");

        let n_threads = 20;
        let barrier = Arc::new(Barrier::new(n_threads + 1));
        let mut handles = Vec::with_capacity(n_threads);

        for _ in 0..n_threads {
            let c_barrier = barrier.clone();
            // Each thread attaches to the shared memory and verifies the data
            handles.push(thread::spawn(move || {
                // Ensure that all threads start simultaneously
                c_barrier.wait();
                let attached_hive: Hive<i32, Semaphore> =
                    Hive::attach(key).expect("Failed to attach to shared memory");
                assert_eq!(
                    attached_hive.read(),
                    initial_data,
                    "Data mismatch in attached shared memory"
                );
            }));
        }

        // Wait for all threads to be ready, then release them at once
        barrier.wait();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }
}
