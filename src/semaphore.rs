use crate::{crash::CortexError, CortexResult, CortexSync};
use std::ffi::{CString, NulError};

fn get_name(shmem_key: i32) -> Result<CString, NulError> {
    let name = CString::new(format!("cortex_semaphore_{}", shmem_key))?;
    Ok(name)
}

#[allow(dead_code)]
/// Set of pre-defined permissions to use
pub enum SemaphorePermission {
    OwnerOnly,
    OwnerAndGroup,
    ReadWriteForOthers,
    ReadOnlyForOthers,
    FullAccessForEveryone,
    Custom(libc::mode_t),
}

impl SemaphorePermission {
    fn as_mode(&self) -> libc::mode_t {
        match self {
            SemaphorePermission::OwnerOnly => libc::S_IRWXU,
            SemaphorePermission::OwnerAndGroup => libc::S_IRWXU | libc::S_IRWXG,
            SemaphorePermission::ReadWriteForOthers => {
                libc::S_IRWXU | libc::S_IRWXG | libc::S_IROTH | libc::S_IWOTH
            }
            SemaphorePermission::ReadOnlyForOthers => libc::S_IRWXU | libc::S_IRWXG | libc::S_IROTH,
            SemaphorePermission::FullAccessForEveryone => {
                libc::S_IRWXU | libc::S_IRWXG | libc::S_IROTH | libc::S_IWOTH | libc::S_IXOTH
            }
            SemaphorePermission::Custom(mode) => *mode,
        }
    }
}

/// Lock that uses a single semaphore for both read and write access
#[derive(Debug)]
pub struct Semaphore {
    semaphore: *mut libc::sem_t,
    name: CString,
    is_owner: bool,
}

pub struct SemaphoreSettings {
    pub mode: SemaphorePermission,
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        if !self.is_owner {
            return;
        }
        unsafe {
            if libc::sem_close(self.semaphore) == -1 {
                tracing::error!("Error during sem_close");
            };
            if libc::sem_unlink(self.name.as_ptr()) == -1 {
                tracing::error!("Error during sem_unlink");
            }
        }
    }
}

impl CortexSync for Semaphore {
    type Settings = SemaphoreSettings;

    fn new(cortex_key: i32, settings: Option<&Self::Settings>) -> CortexResult<Self> {
        let permission = if let Some(settings) = settings {
            settings.mode.as_mode()
        } else {
            // Use most restrictive mode as default
            SemaphorePermission::OwnerOnly.as_mode()
        };
        let name = match get_name(cortex_key) {
            Ok(name) => name,
            Err(_) => return Err(CortexError::new_clean("CString NulError")),
        };
        let name_ptr = name.as_ptr();
        let semaphore =
            unsafe { libc::sem_open(name_ptr, libc::O_EXCL | libc::O_CREAT, permission, 1) };
        if semaphore == libc::SEM_FAILED {
            return Err(CortexError::new_clean("Error during sem_open"));
        }
        Ok(Self {
            semaphore,
            name,
            is_owner: true,
        })
    }
    fn attach(cortex_key: i32) -> CortexResult<Self> {
        let name = match get_name(cortex_key) {
            Ok(name) => name,
            Err(_) => return Err(CortexError::new_clean("CString NulError")),
        };
        let name_ptr = name.as_ptr();
        let semaphore = unsafe { libc::sem_open(name_ptr, 0, 0, 0) };
        if semaphore == libc::SEM_FAILED {
            return Err(CortexError::new_clean("Error during sem_open"));
        }
        Ok(Self {
            semaphore,
            name,
            is_owner: false,
        })
    }
    fn read_lock(&self) -> CortexResult<()> {
        if unsafe { libc::sem_wait(self.semaphore) } == -1 {
            Err(CortexError::new_clean("Error during sem_wait"))
        } else {
            Ok(())
        }
    }
    fn write_lock(&self) -> CortexResult<()> {
        if unsafe { libc::sem_wait(self.semaphore) } == -1 {
            Err(CortexError::new_clean("Error during sem_wait"))
        } else {
            Ok(())
        }
    }
    fn release(&self) -> CortexResult<()> {
        if unsafe { libc::sem_post(self.semaphore) } == -1 {
            Err(CortexError::new_clean("Error during sem_release"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::semaphore::Semaphore;
    use crate::Cortex;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn create_shared_mem() {
        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let cortex: Cortex<_, Semaphore> = Cortex::new(key, data, None).unwrap();
        assert_eq!(cortex.read().unwrap(), 42.0);
    }

    #[test]
    fn attach_to_shared_mem() {
        let key = rand::random::<i32>().abs();
        let data: f64 = 42.0;
        let cortex1: Cortex<_, Semaphore> = Cortex::new(key, data, None).unwrap();
        assert_eq!(cortex1.read().unwrap(), 42.0);

        let cortex2: Cortex<_, Semaphore> = Cortex::attach(key).unwrap();
        assert_eq!(cortex1.read().unwrap(), cortex2.read().unwrap());
    }

    #[test]
    fn multi_thread() {
        let key = rand::random::<i32>().abs();
        let initial_data: i32 = 42;

        // Create a new shared memory segment
        let _cortex: Cortex<_, Semaphore> =
            Cortex::new(key, initial_data, None).expect("Failed to create shared memory");

        let n_threads = 20;
        let barrier = Arc::new(Barrier::new(n_threads + 1));
        let mut handles = Vec::with_capacity(n_threads);

        for _ in 0..n_threads {
            let c_barrier = barrier.clone();
            // Each thread attaches to the shared memory and verifies the data
            handles.push(thread::spawn(move || {
                // Ensure that all threads start simultaneously
                c_barrier.wait();
                let attached_cortex: Cortex<i32, Semaphore> =
                    Cortex::attach(key).expect("Failed to attach to shared memory");
                assert_eq!(
                    attached_cortex.read().unwrap(),
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
