/// Get the current working directory of a process by PID.
///
/// macOS: uses `proc_pidinfo(PROC_PIDVNODEPATHINFO)` FFI.
/// Linux: reads `/proc/<pid>/cwd` symlink.
#[must_use]
pub fn get_process_cwd(pid: u32) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        get_process_cwd_macos(pid)
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{pid}/cwd"))
            .ok()
            .and_then(|p| p.to_str().map(String::from))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = pid;
        None
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn get_process_cwd_macos(pid: u32) -> Option<String> {
    use std::os::raw::c_int;

    unsafe extern "C" {
        fn proc_pidinfo(
            pid: c_int,
            flavor: c_int,
            arg: u64,
            buffer: *mut std::ffi::c_void,
            buffersize: c_int,
        ) -> c_int;
    }

    const PROC_PIDVNODEPATHINFO: c_int = 9;
    // struct vnode_info = 152 bytes, MAXPATHLEN = 1024
    // struct vnode_info_path = 152 + 1024 = 1176
    // struct proc_vnodepathinfo = 1176 * 2 = 2352 (pvi_cdir + pvi_rdir)
    const VNODE_INFO_SIZE: usize = 152;
    const MAXPATHLEN: usize = 1024;
    const BUF_SIZE: usize = (VNODE_INFO_SIZE + MAXPATHLEN) * 2;

    let mut buf = vec![0u8; BUF_SIZE];

    // SAFETY: proc_pidinfo is a stable macOS API (libproc.h).
    // We pass a correctly-sized buffer and check the return value.
    let ret = unsafe {
        proc_pidinfo(
            pid as c_int,
            PROC_PIDVNODEPATHINFO,
            0,
            buf.as_mut_ptr().cast(),
            BUF_SIZE as c_int,
        )
    };

    if ret <= 0 {
        return None;
    }

    // CWD path is in pvi_cdir.vip_path, at offset VNODE_INFO_SIZE
    let path_bytes = &buf[VNODE_INFO_SIZE..VNODE_INFO_SIZE + MAXPATHLEN];
    let nul = path_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(MAXPATHLEN);
    let path = std::str::from_utf8(&path_bytes[..nul]).ok()?;

    if path.is_empty() {
        None
    } else {
        Some(path.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_cwd() {
        let pid = std::process::id();
        let cwd = get_process_cwd(pid);
        assert!(cwd.is_some(), "should detect own process CWD");
        let cwd = cwd.unwrap();
        assert!(!cwd.is_empty());
        assert!(cwd.starts_with('/'), "CWD should be absolute: {cwd}");
    }

    #[test]
    fn invalid_pid_returns_none() {
        assert!(get_process_cwd(99_999_999).is_none());
    }
}
