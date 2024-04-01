use crate::{hive_error::HiveError, HiveResult, HiveSync};
use std::ffi::CString;

fn get_name(shmem_key: i32) -> HiveResult<CString> {
    let name = CString::new(format!("hive_sem_{}", shmem_key))?;
    Ok(name)
}

#[allow(dead_code)]
enum SemaphorePermission {
    OwnerOnly,
    OwnerAndGroup,
    ReadWriteForOthers,
    ReadOnlyForOthers,
    FullAccessForEveryone,
    Custom(libc::mode_t),
}

impl SemaphorePermission {
    fn into_mode(self) -> libc::mode_t {
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
            SemaphorePermission::Custom(mode) => mode,
        }
    }
}

/// Lock that uses a single semaphore for both read and write access
pub(super) struct Semaphore {
    semaphore: *mut libc::sem_t,
    name: CString,
    is_owner: bool,
}

pub(super) struct SemaphoreSettings {
    mode: SemaphorePermission,
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
    type Settings = SemaphoreSettings;

    fn new(shmem_key: i32, settings: Option<Self::Settings>) -> HiveResult<Self> {
        let permission = if let Some(settings) = settings {
            settings.mode
        } else {
            // Use most restrictive mode as default
            SemaphorePermission::OwnerOnly
        };
        let name = get_name(shmem_key)?;
        let name_ptr = name.as_ptr();
        let semaphore = unsafe {
            libc::sem_open(
                name_ptr,
                libc::O_EXCL | libc::O_CREAT,
                permission.into_mode(),
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
        let name = get_name(shmem_key)?;
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
