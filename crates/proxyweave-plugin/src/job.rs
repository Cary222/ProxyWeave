//! Win32 Job Object RAII abstraction for Managed plugins.
//!
//! Provides kernel-level process lifecycle binding: when the JobObject handle
//! is closed (or when the host process terminates/crashes), Windows automatically
//! terminates all processes assigned to the job (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).

use std::io;

#[cfg(windows)]
use std::mem;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

#[derive(Debug)]
pub struct JobObject {
    #[cfg(windows)]
    handle: HANDLE,
}

// Windows HANDLE can be safely sent across threads when appropriately managed.
#[cfg(windows)]
unsafe impl Send for JobObject {}
#[cfg(windows)]
unsafe impl Sync for JobObject {}

impl JobObject {
    /// Creates a new Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    pub fn create() -> io::Result<Self> {
        #[cfg(windows)]
        unsafe {
            let handle = CreateJobObjectW(ptr::null_mut(), ptr::null());
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let set_res = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if set_res == 0 {
                let err = io::Error::last_os_error();
                CloseHandle(handle);
                return Err(err);
            }

            Ok(Self { handle })
        }

        #[cfg(not(windows))]
        {
            Ok(Self {})
        }
    }

    /// Assigns a process to this Job Object by its raw OS handle.
    ///
    /// # Safety
    /// Caller must ensure that `process_handle` is a valid Windows process handle.
    #[cfg(windows)]
    pub unsafe fn assign_process_raw(&self, process_handle: HANDLE) -> io::Result<()> {
        let res = AssignProcessToJobObject(self.handle, process_handle);
        if res == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Assigns a std::process::Child to this Job Object.
    pub fn assign_std_child(&self, child: &std::process::Child) -> io::Result<()> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            unsafe { self.assign_process_raw(child.as_raw_handle() as HANDLE) }
        }

        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(())
        }
    }

    /// Assigns a tokio::process::Child to this Job Object.
    pub fn assign_tokio_child(&self, child: &tokio::process::Child) -> io::Result<()> {
        #[cfg(windows)]
        {
            if let Some(raw_handle) = child.raw_handle() {
                unsafe { self.assign_process_raw(raw_handle as HANDLE) }
            } else {
                Err(io::Error::other(
                    "Tokio child process does not have a valid raw handle",
                ))
            }
        }

        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(())
        }
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            if !self.handle.is_null() {
                CloseHandle(self.handle);
                self.handle = ptr::null_mut();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[cfg(windows)]
    #[test]
    fn test_job_object_kills_child_on_drop() {
        let job = JobObject::create().expect("create job object");

        let start = Instant::now();
        let mut child = std::process::Command::new("powershell")
            .args(["-Command", "Start-Sleep -Seconds 30"])
            .spawn()
            .expect("spawn powershell child");

        job.assign_std_child(&child).expect("assign child to job");

        // Dropping job object must cause child process to be immediately terminated by Windows
        drop(job);

        let status = child.wait().expect("wait for child exit");
        let elapsed = start.elapsed();

        // The process should terminate in less than 5 seconds (normally under 50ms) rather than 30 seconds
        assert!(
            elapsed.as_secs() < 5,
            "Child process should have been killed on job drop, but took {:?}",
            elapsed
        );
        println!(
            "Child terminated on job drop in {:?} with status: {:?}",
            elapsed, status
        );
    }
}
