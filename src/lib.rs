mod hive_error;

use hive_error::HiveError;
use libc;
use std::{ffi::CString, fmt::Display};
use tracing;

type Result<T> = std::result::Result<T, HiveError>;

/// Attempt to clean up a segment of shared memory
fn try_clear_mem(id: i32) -> Result<()> {
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

pub struct Hive<T> {
    key: i32,
    id: i32,
    size: usize,
    is_owner: bool,
    semaphore: *mut libc::sem_t,
    semaphore_name: CString,
    ptr: *mut T,
}

impl<T> Display for Hive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "key: {}, id: {}, size: {}, is_owner: {}",
            self.key, self.id, self.size, self.is_owner
        )
    }
}

impl<T> Hive<T> {
    /// Allocate a new segment of shared memory
    pub fn new(key: i32, data: T) -> Result<Self> {
        // Initialize semaphore
        let semaphore_name = CString::new(format!("hive_mind_{}", key))?;
        let semaphore_name_ptr = semaphore_name.as_ptr();
        let semaphore = unsafe {
            libc::sem_open(
                semaphore_name_ptr,
                libc::O_EXCL | libc::O_CREAT, // create new semaphore and fail if name exists
                libc::S_IRWXU | libc::S_IRWXG, // read/write/execute rights to owner and group
                1,                            // initial value, 1 = unlocked
            )
        };
        if semaphore == libc::SEM_FAILED {
            return Err(HiveError::new_clean("Error during sem_open"));
        }

        // Allocate mem
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
            tracing::trace!("Successfully attached shared memory.");
        }

        // Write to mem
        unsafe {
            ptr.write(data);
        }

        Ok(Self {
            id,
            key,
            size,
            is_owner: true,
            semaphore,
            semaphore_name,
            ptr,
        })
    }
    /// Attempt to attach to an already existing segment of shared memory
    pub fn attach(key: i32) -> Result<Self> {
        let semaphore_name = CString::new(format!("hive_mind_{}", key))?;
        let semaphore_name_ptr = semaphore_name.as_ptr();
        let semaphore = unsafe { libc::sem_open(semaphore_name_ptr, 0, 0, 0) };
        if semaphore == libc::SEM_FAILED {
            return Err(HiveError::new_clean("Error during sem_open"));
        }

        let id = unsafe {
            libc::shmget(key, 0, 0o666) // Size is 0 since we're not creating the segment
        };
        if id == -1 {
            return Err(HiveError::new_clean(format!(
                "Error during shmget for key {}",
                key,
            )));
        } else {
            tracing::trace!("Found shared mem with id {}", id);
        }

        let ptr = unsafe { libc::shmat(id, std::ptr::null_mut(), 0) as *mut T };
        if ptr as isize == -1 {
            return Err(HiveError::new_clean("Error during shmat"));
        } else {
            tracing::trace!("Successfully attached shared mem");
        }

        Ok(Self {
            id,
            key,
            size: std::mem::size_of::<T>(),
            is_owner: false,
            semaphore,
            semaphore_name,
            ptr,
        })
    }
    /// Read from shared mem
    pub fn get_data(&self) -> T {
        unsafe {
            // Wait for semaphore to become available
            libc::sem_wait(self.semaphore);
            let data = self.ptr.read();
            // Free semaphore
            libc::sem_post(self.semaphore);
            data
        }
    }
    /// Write data to shared mem
    pub fn set_data(&self, data: T) {
        unsafe {
            // Wait for semaphore to become available
            libc::sem_wait(self.semaphore);
            self.ptr.write(data);
            // Free semaphore
            libc::sem_post(self.semaphore);
        }
    }
}

/// Drop a segment of shared memory and clean up its semaphore
impl<T> Drop for Hive<T> {
    fn drop(&mut self) {
        if !self.is_owner {
            return;
        }
        unsafe {
            if libc::sem_close(self.semaphore) == -1 {
                tracing::error!("Error during sem_close");
            };
            if libc::sem_unlink(self.semaphore_name.as_ptr()) == -1 {
                tracing::error!("Error during sem_unlink");
            }
        }
        if let Err(err) = try_clear_mem(self.id) {
            tracing::error!("{err}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn create_shared_mem() {
        use crate::*;

        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let hive = Hive::new(key, data).unwrap();
        assert_eq!(hive.get_data(), 42.0);
    }

    #[test]
    fn attach_to_shared_mem() {
        use crate::*;

        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let hive = Hive::new(key, data).unwrap();
        assert_eq!(hive.get_data(), 42.0);

        let hive2 = Hive::attach(key).unwrap();
        assert_eq!(hive.get_data(), hive2.get_data());
    }

    #[test]
    fn multi_thread() {
        let key = rand::random::<i32>().abs();
        let initial_data: i32 = 42;

        // Create a new shared memory segment
        let _hive = Hive::new(key, initial_data).expect("Failed to create shared memory");

        let n_threads = 20;
        let barrier = Arc::new(Barrier::new(n_threads + 1));
        let mut handles = Vec::with_capacity(n_threads);

        for _ in 0..n_threads {
            let c_barrier = barrier.clone();
            // Each thread attaches to the shared memory and verifies the data
            handles.push(thread::spawn(move || {
                // Ensure that all threads start simultaneously
                c_barrier.wait();
                let attached_hive: Hive<i32> =
                    Hive::attach(key).expect("Failed to attach to shared memory");
                assert_eq!(
                    attached_hive.get_data(),
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
