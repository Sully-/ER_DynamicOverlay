use anyhow::{Context, Result};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleBaseNameW};
use windows::Win32::System::Threading::{
    IsWow64Process, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

pub struct ProcessInfo {
    pub pid: u32,
    pub image_name: String,
}

impl ProcessInfo {
    pub fn open(pid: u32) -> Result<Self> {
        let handle = open_query_handle(pid)?;
        unsafe { CloseHandle(handle)? };
        Ok(Self {
            pid,
            image_name: format!("pid:{pid}"),
        })
    }
}

pub fn find_process_by_name(name: &str) -> Result<Option<ProcessInfo>> {
    let name_lower = name.to_lowercase();
    unsafe {
        let snap =
            CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).context("CreateToolhelp32Snapshot")?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let exe = utf16_until_nul(&entry.szExeFile);
                if exe.eq_ignore_ascii_case(&name_lower) {
                    let _ = CloseHandle(snap);
                    return Ok(Some(ProcessInfo {
                        pid: entry.th32ProcessID,
                        image_name: exe,
                    }));
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    Ok(None)
}

pub fn is_process_x64(pid: u32) -> Result<bool> {
    use windows::core::BOOL;
    use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .context("OpenProcess for WOW64 check")?;
        let mut wow64 = BOOL::from(false);
        IsWow64Process(handle, &mut wow64).context("IsWow64Process")?;
        let _ = CloseHandle(handle);
        Ok(!wow64.as_bool())
    }
}

pub fn list_loaded_modules(pid: u32) -> Result<Vec<String>> {
    unsafe {
        let handle = open_query_handle(pid)?;
        let mut modules = [HMODULE::default(); 512];
        let mut needed = 0u32;
        EnumProcessModules(
            handle,
            modules.as_mut_ptr(),
            (modules.len() * std::mem::size_of::<HMODULE>()) as u32,
            &mut needed,
        )
        .context("EnumProcessModules")?;
        let count = (needed as usize) / std::mem::size_of::<HMODULE>();
        let mut names = Vec::new();
        let mut buf = [0u16; 260];
        for &module in modules.iter().take(count) {
            let len = GetModuleBaseNameW(handle, Some(module), &mut buf);
            if len > 0 {
                names.push(String::from_utf16_lossy(&buf[..len as usize]));
            }
        }
        let _ = CloseHandle(handle);
        Ok(names)
    }
}

fn open_query_handle(pid: u32) -> Result<HANDLE> {
    unsafe {
        OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).context("OpenProcess")
    }
}

fn utf16_until_nul(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_helper() {
        let buf = [b'e' as u16, b'r' as u16, 0, b'x' as u16];
        assert_eq!(utf16_until_nul(&buf), "er");
    }
}
