use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use anyhow::{bail, Context, Result};
use tracing::{info, warn};
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetExitCodeThread, OpenProcess, WaitForSingleObject, INFINITE,
    PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_WRITE,
};

/// Transparent documented injection: remote thread calling LoadLibraryW with DLL path.
pub fn inject_loadlibrary(pid: u32, dll_path: &Path) -> Result<()> {
    let wide: Vec<u16> = OsStr::new(dll_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_bytes = wide.len() * 2;

    unsafe {
        let process = OpenProcess(
            PROCESS_CREATE_THREAD
                | PROCESS_QUERY_INFORMATION
                | PROCESS_VM_OPERATION
                | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .context("OpenProcess for injection")?;
        info!("OpenProcess OK pid={pid}");

        let remote_mem = VirtualAllocEx(
            process,
            None,
            wide_bytes,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if remote_mem.is_null() {
            let _ = CloseHandle(process);
            bail!("VirtualAllocEx returned null");
        }
        info!("VirtualAllocEx OK addr={remote_mem:p}");

        WriteProcessMemory(process, remote_mem, wide.as_ptr() as _, wide_bytes, None)
            .context("WriteProcessMemory")?;
        info!("Wrote DLL path ({wide_bytes} bytes) to remote process");

        let kernel32 = GetModuleHandleA(windows::core::s!("kernel32.dll"))
            .context("GetModuleHandleA kernel32")?;

        let load_library = GetProcAddress(kernel32, windows::core::s!("LoadLibraryW"))
            .ok_or_else(|| anyhow::anyhow!("GetProcAddress LoadLibraryW failed"))?;
        info!("LoadLibraryW at {load_library:?}");

        let thread = CreateRemoteThread(
            process,
            None,
            0,
            Some(std::mem::transmute::<
                unsafe extern "system" fn() -> isize,
                unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
            >(load_library)),
            Some(remote_mem),
            0,
            None,
        )
        .context("CreateRemoteThread")?;
        info!("CreateRemoteThread OK");

        let wait = WaitForSingleObject(thread, INFINITE);
        if wait != WAIT_OBJECT_0 {
            let _ = CloseHandle(thread);
            let _ = VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
            let _ = CloseHandle(process);
            bail!("WaitForSingleObject returned {wait:?}");
        }

        // The remote thread runs LoadLibraryW; its exit code is the (truncated)
        // returned HMODULE. Zero means LoadLibraryW returned NULL → the DLL was
        // not loaded, so don't report a false success.
        let mut exit_code: u32 = 0;
        let exit_code_ok = GetExitCodeThread(thread, &mut exit_code).is_ok();

        let _ = CloseHandle(thread);
        let _ = VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
        let _ = CloseHandle(process);

        if exit_code_ok {
            if exit_code == 0 {
                bail!("LoadLibraryW returned NULL in target process (DLL not loaded)");
            }
            info!("Remote LoadLibraryW returned module handle (low32=0x{exit_code:08x})");
        } else {
            warn!("Could not read remote thread exit code; load status unverified");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_wide_null_terminated() {
        let p = Path::new("C:\\test\\er_overlay.dll");
        let wide: Vec<u16> = OsStr::new(p)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        assert_eq!(wide.last(), Some(&0));
    }
}
