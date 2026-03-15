//! OS-level helpers: memory management and process cleanup.

/// Ask glibc to return free heap pages to the OS.
/// Cost: ~1μs per call. Prevents RSS growth from transient allocations.
#[cfg(target_os = "linux")]
pub fn trim_heap() {
    // SAFETY: malloc_trim(0) is always safe, releases free heap pages to OS.
    unsafe {
        libc::malloc_trim(0);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn trim_heap() {}

/// Limit glibc malloc arenas to avoid VSZ bloat from per-thread arenas.
/// Each arena reserves ~64MB of virtual address space (PROT_NONE, no RSS cost)
/// but inflates VSZ. With 2 arenas (main + 1 worker) VSZ stays bounded.
#[cfg(target_os = "linux")]
pub fn limit_malloc_arenas() {
    const M_ARENA_MAX: i32 = -8;
    const ARENA_COUNT: i32 = 2;
    // SAFETY: safe when called before threads spawn.
    unsafe {
        libc::mallopt(M_ARENA_MAX, ARENA_COUNT);
    }
}

#[cfg(target_os = "linux")]
mod child_pids {
    use std::sync::Mutex;

    static PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    pub fn register(pid: u32) {
        PIDS.lock().unwrap_or_else(|e| e.into_inner()).push(pid);
    }

    pub fn drain() -> Vec<u32> {
        std::mem::take(&mut *PIDS.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

#[cfg(target_os = "linux")]
pub fn register_child_pid(pid: u32) {
    child_pids::register(pid);
}

#[cfg(not(target_os = "linux"))]
pub fn register_child_pid(_pid: u32) {}

/// Reap finished child processes to prevent zombie accumulation.
/// Only reaps PIDs registered via `register_child_pid()`.
#[cfg(target_os = "linux")]
pub fn reap_zombies() {
    for pid in child_pids::drain() {
        loop {
            // SAFETY: valid PID, non-blocking, null status pointer is allowed.
            let ret = unsafe { libc::waitpid(pid as i32, std::ptr::null_mut(), libc::WNOHANG) };
            if ret == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            if ret == 0 {
                // Still running — re-register for next iteration
                child_pids::register(pid);
            }
            break;
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn reap_zombies() {}
