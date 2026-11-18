// Windows readings via raw kernel32/ntdll FFI (no extra crate).
// Mirrors host_pressure/_core.py Windows branch.

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    #[repr(C)]
    struct SystemPowerStatus {
        ac_line_status: u8,
        battery_flag: u8,
        battery_life_percent: u8,
        system_status_flag: u8,
        battery_life_time: u32,
        battery_full_life_time: u32,
    }

    #[repr(C)]
    struct SystemInfo {
        w_processor_architecture: u16,
        w_reserved: u16,
        dw_page_size: u32,
        lp_minimum_application_address: *mut std::ffi::c_void,
        lp_maximum_application_address: *mut std::ffi::c_void,
        dw_active_processor_mask: usize,
        dw_number_of_processors: u32,
        dw_processor_type: u32,
        dw_allocation_granularity: u32,
        w_processor_level: u16,
        w_processor_revision: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(
            idle: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> i32;
        fn GlobalMemoryStatusEx(lp: *mut MemoryStatusEx) -> i32;
        fn GetTickCount64() -> u64;
        fn GetDiskFreeSpaceExW(
            dir: *const u16,
            free_to_caller: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
        fn GetSystemPowerStatus(lp: *mut SystemPowerStatus) -> i32;
        fn GetSystemInfo(lp: *mut SystemInfo);
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQuerySystemInformation(
            class: u32,
            buffer: *mut std::ffi::c_void,
            length: u32,
            retlen: *mut u32,
        ) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsString::from(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn cpu_ticks() -> Option<Vec<u64>> {
        unsafe {
            let (mut idle, mut kernel, mut user) = (0u64, 0u64, 0u64);
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
                return None;
            }
            Some(vec![idle, kernel, user])
        }
    }

    pub fn uptime_seconds() -> Option<f64> {
        unsafe { Some(GetTickCount64() as f64 / 1000.0) }
    }

    pub fn boot_time() -> Option<f64> {
        let u = uptime_seconds()?;
        Some(crate::core::now_unix_for_win() - u)
    }

    pub fn memory() -> (Option<f64>, Option<i64>, Option<i64>, Option<i64>) {
        unsafe {
            let mut st: MemoryStatusEx = std::mem::zeroed();
            st.dw_length = std::mem::size_of::<MemoryStatusEx>() as u32;
            if GlobalMemoryStatusEx(&mut st) == 0 {
                return (None, None, None, None);
            }
            let total = st.ull_total_phys as i64;
            let used = total - (st.ull_avail_phys as i64);
            let load = (st.dw_memory_load as f64 * 10.0).round() / 10.0;
            let commit = (st.ull_total_page_file as i64) - (st.ull_avail_page_file as i64);
            (Some(load), Some(total), Some(used), Some(commit))
        }
    }

    pub fn pagefile() -> (Option<i64>, Option<i64>) {
        unsafe {
            let size: u32 = 65536;
            let mut buffer = vec![0u8; size as usize];
            let mut ret: u32 = 0;
            let status = NtQuerySystemInformation(
                18,
                buffer.as_mut_ptr() as *mut _,
                size,
                &mut ret,
            );
            if status != 0 {
                return (None, None);
            }
            let mut page: u32 = 4096;
            let mut si: SystemInfo = std::mem::zeroed();
            GetSystemInfo(&mut si);
            if si.dw_page_size != 0 {
                page = si.dw_page_size;
            }
            let (mut used, mut total) = (0i64, 0i64);
            let mut offset = 0usize;
            while offset + 16 <= ret as usize {
                let next = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap()) as usize;
                let total_pages = u32::from_le_bytes(buffer[offset + 4..offset + 8].try_into().unwrap()) as i64;
                let in_use = u32::from_le_bytes(buffer[offset + 8..offset + 12].try_into().unwrap()) as i64;
                total += total_pages * (page as i64);
                used += in_use * (page as i64);
                if next == 0 {
                    break;
                }
                offset += next;
            }
            (Some(used), Some(total))
        }
    }

    pub fn battery() -> Option<crate::core::Battery> {
        unsafe {
            let mut st: SystemPowerStatus = std::mem::zeroed();
            if GetSystemPowerStatus(&mut st) == 0 {
                return None;
            }
            if st.battery_life_percent == 255 || st.battery_life_percent == 128 {
                return None;
            }
            let state = match st.ac_line_status {
                0 => "discharging",
                1 => "charging",
                _ => "full",
            }
            .to_string();
            let seconds = if st.battery_life_time == 0xFFFFFFFF || st.battery_life_time == 0 {
                None
            } else {
                Some(st.battery_life_time as i64)
            };
            Some(crate::core::Battery {
                percent: Some(st.battery_life_percent as i64),
                state: Some(state),
                seconds_left: seconds,
            })
        }
    }

    pub fn disks(mounts: &[String]) -> Vec<crate::core::Disk> {
        let mut out = Vec::new();
        for m in mounts {
            unsafe {
                let w = wide(m);
                let (mut caller, mut total, mut free) = (0u64, 0u64, 0u64);
                let ok = GetDiskFreeSpaceExW(w.as_ptr(), &mut caller, &mut total, &mut free);
                if ok == 0 || total == 0 {
                    out.push(crate::core::Disk {
                        mount: m.clone(),
                        total_bytes: None,
                        used_bytes: None,
                        free_bytes: None,
                        percent: None,
                    });
                    continue;
                }
                let used = total - free;
                let pct = ((used as f64) * 100.0 / (total as f64) * 10.0).round() / 10.0;
                out.push(crate::core::Disk {
                    mount: m.clone(),
                    total_bytes: Some(total as i64),
                    used_bytes: Some(used as i64),
                    free_bytes: Some(free as i64),
                    percent: Some(pct),
                });
            }
        }
        out
    }
}

#[cfg(target_os = "windows")]
pub use imp::{battery as windows_battery, boot_time as windows_boot_time, cpu_ticks as windows_cpu_ticks, disks as windows_disks, memory as windows_memory, pagefile as windows_pagefile, uptime_seconds as windows_uptime_seconds};
