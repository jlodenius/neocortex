use crate::{hive_error::HiveError, HiveResult, HiveSync};
use std::ffi::CString;

#[allow(dead_code)]
enum SemaphorePermission {
    OwnerOnly,
    OwnerAndGroup,
    ReadWriteForOthers,
    ReadOnlyForOthers,
    FullAccessForEveryone,
}

impl SemaphorePermission {
    fn to_mode(self) -> libc::mode_t {
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
        }
    }
}

/// Lock that uses a single semaphore for both read and write access
pub(super) struct Semaphore {
    semaphore: *mut libc::sem_t,
    name: CString,
    is_owner: bool,
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

impl HiveSync for Semaphore {
    fn new(shmem_key: i32) -> HiveResult<Self> {
        let name = CString::new(format!("hive_mind_sem_{}", shmem_key))?;
        let name_ptr = name.as_ptr();
        let semaphore = unsafe {
            libc::sem_open(
                name_ptr,
                libc::O_EXCL | libc::O_CREAT,
                SemaphorePermission::OwnerAndGroup.to_mode(),
                1,
            )
        };
        if semaphore == libc::SEM_FAILED {
            return Err(HiveError::new_clean("Error during sem_open"));
        }
        Ok(Self {
            semaphore,
            name,
            is_owner: true,
        })
    }
    fn attach(shmem_key: i32) -> HiveResult<Self> {
        let name = CString::new(format!("hive_mind_sem_{}", shmem_key))?;
        let name_ptr = name.as_ptr();
        let semaphore = unsafe { libc::sem_open(name_ptr, 0, 0, 0) };
        if semaphore == libc::SEM_FAILED {
            return Err(HiveError::new_clean("Error during sem_open"));
        }
        Ok(Self {
            semaphore,
            name,
            is_owner: false,
        })
    }
    fn read_lock(&self) {
        unsafe {
            libc::sem_wait(self.semaphore);
        }
    }
    fn write_lock(&self) {
        unsafe {
            libc::sem_wait(self.semaphore);
        }
    }
    fn release(&self) {
        unsafe {
            libc::sem_post(self.semaphore);
        }
    }
}
