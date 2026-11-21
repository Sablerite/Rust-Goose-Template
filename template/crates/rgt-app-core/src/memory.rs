//! Process memory readings, for the memory tests and the contest.
//!
//! "Commit" is memory the process has asked the system to back with RAM or
//! the page file: what a growing data structure costs. Address space that is
//! only reserved (Goose reserves 2 GiB per data stack) is not counted.
//!
//! Readings are available on Windows; elsewhere every function returns `None`.

use std::process::Child;

/// Memory committed by this process right now, in bytes.
pub fn current_commit() -> Option<u64> {
    imp::counters_of_self().map(|c| c.current)
}

/// The most memory this process has had committed at once, in bytes.
pub fn peak_commit() -> Option<u64> {
    imp::counters_of_self().map(|c| c.peak)
}

/// The peak commit of a child process. Works after the child has exited, as
/// long as `child` has not been dropped (it keeps the process handle open).
pub fn peak_commit_of(child: &Child) -> Option<u64> {
    imp::counters_of_child(child).map(|c| c.peak)
}

struct Counters {
    current: u64,
    peak: u64,
}

#[cfg(windows)]
mod imp {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    use super::Counters;

    fn counters(process: HANDLE) -> Option<Counters> {
        let mut info = PROCESS_MEMORY_COUNTERS_EX::default();
        let size = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX>()).ok()?;
        // SAFETY: `info` is a valid, writable PROCESS_MEMORY_COUNTERS_EX and
        // `size` is its exact size, which tells Windows to fill the _EX form.
        let ok = unsafe {
            GetProcessMemoryInfo(
                process,
                (&raw mut info).cast::<PROCESS_MEMORY_COUNTERS>(),
                size,
            )
        };
        (ok != 0).then_some(Counters {
            current: info.PrivateUsage as u64,
            peak: info.PeakPagefileUsage as u64,
        })
    }

    pub(super) fn counters_of_self() -> Option<Counters> {
        // SAFETY: GetCurrentProcess has no preconditions; it returns a
        // pseudo-handle that needs no closing.
        let me = unsafe { GetCurrentProcess() };
        counters(me)
    }

    pub(super) fn counters_of_child(child: &Child) -> Option<Counters> {
        counters(child.as_raw_handle())
    }
}

#[cfg(not(windows))]
mod imp {
    use std::process::Child;

    use super::Counters;

    pub(super) fn counters_of_self() -> Option<Counters> {
        None
    }

    pub(super) fn counters_of_child(_child: &Child) -> Option<Counters> {
        None
    }
}
