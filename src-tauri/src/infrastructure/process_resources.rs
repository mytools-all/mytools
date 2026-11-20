use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct ServiceProcess {
    pub name: String,
    pub executable: String,
}

#[cfg(windows)]
pub fn private_working_set(pid: u32) -> Option<u64> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX2},
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut counters: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
        let size = std::mem::size_of_val(&counters) as u32;
        counters.cb = size;
        let ok = K32GetProcessMemoryInfo(
            handle,
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX2).cast(),
            size,
        );
        CloseHandle(handle);
        (ok != 0).then_some(counters.PrivateWorkingSetSize as u64)
    }
}

#[cfg(not(windows))]
pub fn private_working_set(_pid: u32) -> Option<u64> {
    None
}

fn executable_from_command(command: &str) -> Option<String> {
    let value = command.trim();
    let path = if let Some(rest) = value.strip_prefix('"') {
        rest.split_once('"')?.0
    } else {
        let end = value.to_ascii_lowercase().find(".exe")? + 4;
        &value[..end]
    };
    (!path.is_empty()).then(|| path.to_string())
}

#[cfg(windows)]
pub fn service_processes(pids: &HashSet<u32>) -> HashMap<u32, ServiceProcess> {
    use windows_sys::Win32::System::Services::*;
    struct ServiceHandle(SC_HANDLE);
    impl Drop for ServiceHandle {
        fn drop(&mut self) {
            unsafe {
                CloseServiceHandle(self.0);
            }
        }
    }
    unsafe fn wide(pointer: *const u16) -> String {
        let mut size = 0;
        while size < 32768 && *pointer.add(size) != 0 {
            size += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(pointer, size))
    }
    let mut result = HashMap::new();
    if pids.is_empty() {
        return result;
    }
    unsafe {
        let manager = OpenSCManagerW(
            std::ptr::null(),
            std::ptr::null(),
            SC_MANAGER_ENUMERATE_SERVICE | SC_MANAGER_CONNECT,
        );
        if manager.is_null() {
            return result;
        }
        let manager = ServiceHandle(manager);
        let mut bytes = 0;
        let mut returned = 0;
        let mut resume = 0;
        EnumServicesStatusExW(
            manager.0,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_ACTIVE,
            std::ptr::null_mut(),
            0,
            &mut bytes,
            &mut returned,
            &mut resume,
            std::ptr::null(),
        );
        if bytes == 0 || bytes > 1024 * 1024 {
            return result;
        }
        let mut buffer = vec![0usize; (bytes as usize).div_ceil(std::mem::size_of::<usize>())];
        resume = 0;
        if EnumServicesStatusExW(
            manager.0,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_ACTIVE,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * std::mem::size_of::<usize>()) as u32,
            &mut bytes,
            &mut returned,
            &mut resume,
            std::ptr::null(),
        ) == 0
        {
            return result;
        }
        let services = std::slice::from_raw_parts(
            buffer.as_ptr().cast::<ENUM_SERVICE_STATUS_PROCESSW>(),
            returned as usize,
        );
        for service in services {
            let pid = service.ServiceStatusProcess.dwProcessId;
            if !pids.contains(&pid) || service.lpServiceName.is_null() {
                continue;
            }
            let handle = OpenServiceW(manager.0, service.lpServiceName, SERVICE_QUERY_CONFIG);
            if handle.is_null() {
                continue;
            }
            let handle = ServiceHandle(handle);
            let mut needed = 0;
            QueryServiceConfigW(handle.0, std::ptr::null_mut(), 0, &mut needed);
            if needed == 0 || needed > 65536 {
                continue;
            }
            let mut config = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
            if QueryServiceConfigW(
                handle.0,
                config.as_mut_ptr().cast(),
                (config.len() * std::mem::size_of::<usize>()) as u32,
                &mut needed,
            ) == 0
            {
                continue;
            }
            let config = &*config.as_ptr().cast::<QUERY_SERVICE_CONFIGW>();
            if config.lpBinaryPathName.is_null() {
                continue;
            }
            if let Some(executable) = executable_from_command(&wide(config.lpBinaryPathName)) {
                result.insert(
                    pid,
                    ServiceProcess {
                        name: wide(service.lpServiceName),
                        executable,
                    },
                );
            }
        }
    }
    result
}

#[cfg(not(windows))]
pub fn service_processes(_pids: &HashSet<u32>) -> HashMap<u32, ServiceProcess> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_executable_does_not_include_arguments() {
        assert_eq!(
            executable_from_command(r#""C:\Program Files\Everything\Everything.exe" -svc"#)
                .as_deref(),
            Some(r"C:\Program Files\Everything\Everything.exe")
        );
        assert_eq!(
            executable_from_command(r"C:\Tools\Everything.exe -svc").as_deref(),
            Some(r"C:\Tools\Everything.exe")
        );
        assert!(executable_from_command("invalid").is_none());
    }
    #[cfg(windows)]
    #[test]
    fn private_memory_can_read_current_process_and_rejects_missing_process() {
        let value = private_working_set(std::process::id())
            .expect("private working set available on supported Windows");
        assert!(value > 0);
        assert!(private_working_set(u32::MAX).is_none());
    }
}
