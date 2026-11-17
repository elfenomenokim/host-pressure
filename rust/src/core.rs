use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Battery {
    pub percent: Option<i64>,
    pub state: Option<String>,
    pub seconds_left: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Disk {
    pub mount: String,
    pub total_bytes: Option<i64>,
    pub used_bytes: Option<i64>,
    pub free_bytes: Option<i64>,
    pub percent: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Reading {
    pub observed_at: Option<String>,
    pub os: Option<String>,
    pub hostname: Option<String>,
    pub ncpu: Option<i64>,
    pub cpu_percent: Option<f64>,
    pub cpu_source: Option<String>,
    pub memory_percent: Option<f64>,
    pub memory_total_bytes: Option<i64>,
    pub memory_used_bytes: Option<i64>,
    pub swap_bytes: Option<i64>,
    pub swap_total_bytes: Option<i64>,
    pub pressure: String,
    pub pressure_source: String,
    pub loadavg: Option<Vec<f64>>,
    pub uptime: Option<String>,
    pub uptime_seconds: Option<f64>,
    pub thermal: Option<String>,
    pub battery: Option<Battery>,
    pub psi: Option<HashMap<String, HashMap<String, f64>>>,
    pub disks: Vec<Disk>,
}

pub fn format_uptime(seconds: Option<f64>) -> Option<String> {
    let s = seconds?;
    if !s.is_finite() || s < 0.0 {
        return None;
    }
    let days = (s / 86400.0).floor() as i64;
    let hours = ((s % 86400.0) / 3600.0).floor() as i64;
    let minutes = ((s % 3600.0) / 60.0).floor() as i64;
    if days != 0 {
        Some(format!("{}d {}h", days, hours))
    } else if hours != 0 {
        Some(format!("{}h {}m", hours, minutes))
    } else if minutes != 0 {
        Some(format!("{}m {}s", minutes, (s % 60.0).floor() as i64))
    } else {
        Some(format!("{}s", s.floor() as i64))
    }
}

fn now_unix() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn iso_now() -> String {
    let now = SystemTime::now();
    let dur = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let micros = (dur.subsec_nanos() / 1000) as u32;
    // days to civil (Howard Hinnant)
    let days = secs.div_euclid(86400);
    let secs_of_day = secs.rem_euclid(86400);
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 }.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    if micros == 0 {
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00", year, m, d, hh, mm, ss)
    } else {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}+00:00",
            year, m, d, hh, mm, ss, micros
        )
    }
}

fn hostname() -> Option<String> {
    #[cfg(unix)]
    {
        unsafe {
            let mut buf = [0 as libc::c_char; 256];
            if libc::gethostname(buf.as_mut_ptr(), buf.len()) == 0 {
                let mut len = 0usize;
                while len < buf.len() && buf[len] != 0 as libc::c_char {
                    len += 1;
                }
                let bytes: Vec<u8> = buf[..len].iter().map(|&c| c as u8).collect();
                if let Ok(s) = String::from_utf8(bytes) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        // fallback: HOSTNAME env
        if let Some(h) = std::env::var_os("HOSTNAME") {
            let s = h.to_string_lossy().into_owned();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
    #[cfg(windows)]
    {
        if let Some(h) = std::env::var_os("COMPUTERNAME") {
            let s = h.to_string_lossy().into_owned();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

fn ncpu() -> Option<i64> {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .ok()
}

fn current_os() -> String {
    #[cfg(target_os = "macos")]
    {
        "darwin".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "linux".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        "windows".to_string()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        std::env::consts::OS.to_string()
    }
}

// ---------------------------------------------------------------------------
// CPU ticks
// ---------------------------------------------------------------------------
fn ticks_expected() -> usize {
    #[cfg(target_os = "macos")]
    {
        4
    }
    #[cfg(target_os = "windows")]
    {
        3
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        8
    }
}

fn tick_limit() -> u64 {
    #[cfg(target_os = "macos")]
    {
        0xFFFFFFFF
    }
    #[cfg(not(target_os = "macos"))]
    {
        0xFFFFFFFFFFFFFFFF
    }
}

// Keep one host send right for resident library callers as well as the CLI.
#[cfg(target_os = "macos")]
fn darwin_host_port() -> libc::mach_port_t {
    static HOST: std::sync::OnceLock<libc::mach_port_t> = std::sync::OnceLock::new();
    *HOST.get_or_init(|| unsafe { libc::mach_host_self() })
}

#[cfg(target_os = "macos")]
fn darwin_cpu_ticks() -> Option<Vec<u64>> {
    unsafe {
        let host = darwin_host_port();
        if host == 0 {
            return None;
        }
        let mut ticks = [0u32; 4];
        let mut count: libc::mach_msg_type_number_t = 4;
        let kr = libc::host_statistics(
            host,
            libc::HOST_CPU_LOAD_INFO,
            ticks.as_mut_ptr() as *mut libc::integer_t,
            &mut count,
        );
        if kr == 0 {
            Some(ticks.iter().map(|&x| x as u64).collect())
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_cpu_ticks() -> Option<Vec<u64>> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }
    let mut out = Vec::new();
    for p in &parts[1..9] {
        out.push(p.parse::<u64>().ok()?);
    }
    Some(out)
}

pub fn now_unix_for_win() -> f64 {
    now_unix()
}

fn cpu_ticks() -> Option<Vec<u64>> {
    #[cfg(target_os = "macos")]
    {
        darwin_cpu_ticks()
    }
    #[cfg(target_os = "linux")]
    {
        linux_cpu_ticks()
    }
    #[cfg(target_os = "windows")]
    {
        crate::win::windows_cpu_ticks()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn idle_ticks(delta: &[u64]) -> u64 {
    #[cfg(target_os = "macos")]
    {
        if delta.len() >= 3 {
            delta[2]
        } else {
            0
        }
    }
    #[cfg(target_os = "windows")]
    {
        if !delta.is_empty() {
            delta[0]
        } else {
            0
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if delta.len() >= 5 {
            delta[3] + delta[4]
        } else {
            0
        }
    }
}

fn busy_percent(delta: &[u64]) -> Option<f64> {
    let total: u128 = delta.iter().map(|&x| x as u128).sum();
    if total == 0 {
        return None;
    }
    #[cfg(target_os = "windows")]
    {
        if delta.len() < 3 {
            return None;
        }
        let total_w = (delta[1] as u128) + (delta[2] as u128);
        if total_w == 0 {
            return None;
        }
        let busy = total_w - (delta[0] as u128).min(total_w);
        if busy == 0 {
            return Some(0.0);
        }
        let pct = (busy as f64) * 100.0 / (total_w as f64);
        return Some((pct * 10.0).round() / 10.0);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let busy = (total as u64).saturating_sub(idle_ticks(delta));
        if busy == 0 {
            return Some(0.0);
        }
        let pct = (busy as f64) * 100.0 / (total as f64);
        Some((pct * 10.0).round() / 10.0)
    }
}

// ---------------------------------------------------------------------------
// Boot / uptime
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
fn darwin_boot_time() -> Option<f64> {
    unsafe {
        let mut tv: libc::timeval = std::mem::zeroed();
        let mut size = std::mem::size_of::<libc::timeval>() as libc::size_t;
        let name = b"kern.boottime\0";
        let r = libc::sysctlbyname(
            name.as_ptr() as *const _,
            &mut tv as *mut _ as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if r == 0 {
            Some(tv.tv_sec as f64)
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_boot_time() -> Option<f64> {
    if let Ok(text) = fs::read_to_string("/proc/stat") {
        for line in text.lines() {
            if line.starts_with("btime ") {
                if let Some(v) = line.split_whitespace().nth(1) {
                    if let Ok(f) = v.parse::<f64>() {
                        return Some(f);
                    }
                }
            }
        }
    }
    if let Ok(text) = fs::read_to_string("/proc/uptime") {
        if let Some(v) = text.split_whitespace().next() {
            if let Ok(u) = v.parse::<f64>() {
                return Some(now_unix() - u);
            }
        }
    }
    None
}

fn boot_time() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        darwin_boot_time()
    }
    #[cfg(target_os = "linux")]
    {
        linux_boot_time()
    }
    #[cfg(target_os = "windows")]
    {
        crate::win::windows_boot_time()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_uptime_seconds() -> Option<f64> {
    fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
}

fn uptime_seconds() -> Option<f64> {
    if let Some(b) = boot_time() {
        let u = now_unix() - b;
        if u >= 0.0 {
            return Some(u);
        }
    }
    #[cfg(target_os = "linux")]
    {
        return linux_uptime_seconds();
    }
    #[cfg(target_os = "windows")]
    {
        return crate::win::windows_uptime_seconds();
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

// ---------------------------------------------------------------------------
// macOS specifics
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
fn sysctl_int(name: &[u8]) -> Option<i32> {
    unsafe {
        let mut v: i32 = 0;
        let mut size = std::mem::size_of::<i32>() as libc::size_t;
        let cname = std::ffi::CString::new(name).ok()?;
        if libc::sysctlbyname(
            cname.as_ptr(),
            &mut v as *mut _ as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == 0
        {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn darwin_mem_total() -> u64 {
    unsafe {
        let mut v: u64 = 0;
        let mut size = std::mem::size_of::<u64>() as libc::size_t;
        let name = b"hw.memsize\0";
        if libc::sysctlbyname(
            name.as_ptr() as *const _,
            &mut v as *mut _ as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == 0
        {
            v
        } else {
            0
        }
    }
}

#[cfg(target_os = "macos")]
fn darwin_page_size() -> u64 {
    unsafe {
        let v = libc::sysconf(libc::_SC_PAGESIZE);
        if v > 0 {
            v as u64
        } else {
            16384
        }
    }
}

#[cfg(target_os = "macos")]
fn darwin_memory() -> (Option<f64>, Option<i64>, Option<i64>) {
    unsafe {
        let total = darwin_mem_total();
        if total == 0 {
            return (None, None, None);
        }
        let host = darwin_host_port();
        if host == 0 {
            return (None, Some(total as i64), None);
        }
        let mut stats: libc::vm_statistics64 = std::mem::zeroed();
        let mut count: libc::mach_msg_type_number_t = libc::HOST_VM_INFO64_COUNT;
        let kr = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            &mut stats as *mut _ as *mut libc::integer_t,
            &mut count,
        );
        if kr != 0 {
            return (None, Some(total as i64), None);
        }
        let page = darwin_page_size();
        let internal = stats.internal_page_count as u64;
        let purgeable = stats.purgeable_count as u64;
        let wire = stats.wire_count as u64;
        let compressor = stats.compressor_page_count as u64;
        let pages = internal.saturating_sub(purgeable).saturating_add(wire).saturating_add(compressor);
        let mut used = pages.saturating_mul(page);
        if used > total {
            used = total;
        }
        let pct = ((used as f64) * 100.0 / (total as f64) * 10.0).round() / 10.0;
        let pct = pct.min(100.0);
        (Some(pct), Some(total as i64), Some(used as i64))
    }
}

#[cfg(target_os = "macos")]
fn darwin_pressure() -> (String, String) {
    match sysctl_int(b"kern.memorystatus_vm_pressure_level") {
        None => ("unknown".to_string(), "unknown".to_string()),
        Some(level) => {
            let s = match level {
                1 => "normal",
                2 => "warning",
                4 => "critical",
                _ => "unknown",
            };
            (s.to_string(), "macos".to_string())
        }
    }
}

#[cfg(target_os = "macos")]
fn darwin_swap() -> (Option<i64>, Option<i64>) {
    unsafe {
        let mut usage: libc::xsw_usage = std::mem::zeroed();
        let mut size = std::mem::size_of::<libc::xsw_usage>() as libc::size_t;
        let name = b"vm.swapusage\0";
        let r = libc::sysctlbyname(
            name.as_ptr() as *const _,
            &mut usage as *mut _ as *mut _,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if r == 0 {
            (Some(usage.xsu_used as i64), Some(usage.xsu_total as i64))
        } else {
            (None, None)
        }
    }
}

// Thermal via NSProcessInfo (objc runtime, no dependency).
#[cfg(target_os = "macos")]
fn darwin_thermal() -> Option<String> {
    unsafe {
        const RTLD_NOW: i32 = 2;
        let objc_name = std::ffi::CString::new("/usr/lib/libobjc.A.dylib").unwrap();
        let found_name = std::ffi::CString::new("/System/Library/Frameworks/Foundation.framework/Foundation").unwrap();
        let objc = libc::dlopen(objc_name.as_ptr(), RTLD_NOW);
        if objc.is_null() {
            return None;
        }
        // Load Foundation so NSProcessInfo exists.
        let f = libc::dlopen(found_name.as_ptr(), RTLD_NOW);
        let _ = f;
        let n_getclass = std::ffi::CString::new("objc_getClass").unwrap();
        let n_sel = std::ffi::CString::new("sel_registerName").unwrap();
        let n_send = std::ffi::CString::new("objc_msgSend").unwrap();
        let p_getclass = libc::dlsym(objc, n_getclass.as_ptr());
        let p_sel = libc::dlsym(objc, n_sel.as_ptr());
        let p_send = libc::dlsym(objc, n_send.as_ptr());
        if p_getclass.is_null() || p_sel.is_null() || p_send.is_null() {
            return None;
        }
        type GetClass = unsafe extern "C" fn(*const i8) -> *mut std::ffi::c_void;
        type SelReg = unsafe extern "C" fn(*const i8) -> *mut std::ffi::c_void;
        let getclass: GetClass = std::mem::transmute(p_getclass);
        let selreg: SelReg = std::mem::transmute(p_sel);
        let cls = getclass(b"NSProcessInfo\0".as_ptr() as *const i8);
        if cls.is_null() {
            return None;
        }
        let sel_pi = selreg(b"processInfo\0".as_ptr() as *const i8);
        let sel_th = selreg(b"thermalState\0".as_ptr() as *const i8);
        type SendId = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        type SendLong = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i64;
        let send_id: SendId = std::mem::transmute(p_send);
        let send_long: SendLong = std::mem::transmute(p_send);
        let info = send_id(cls, sel_pi);
        if info.is_null() {
            return None;
        }
        let state = send_long(info, sel_th);
        match state {
            0 => Some("nominal".to_string()),
            1 => Some("fair".to_string()),
            2 => Some("serious".to_string()),
            3 => Some("critical".to_string()),
            _ => None,
        }
    }
}

// Battery via IOKit (best effort; None when absent).
#[cfg(target_os = "macos")]
fn darwin_battery() -> Option<Battery> {
    unsafe {
        const RTLD_NOW: i32 = 2;
        let cf_name = std::ffi::CString::new("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation").unwrap();
        let io_name = std::ffi::CString::new("/System/Library/Frameworks/IOKit.framework/IOKit").unwrap();
        let cf = libc::dlopen(cf_name.as_ptr(), RTLD_NOW);
        if cf.is_null() {
            return None;
        }
        let iokit = libc::dlopen(io_name.as_ptr(), RTLD_NOW);
        if iokit.is_null() {
            return None;
        }
        let sym = |h: *mut std::ffi::c_void, n: &str| {
            let c = std::ffi::CString::new(n).unwrap();
            libc::dlsym(h, c.as_ptr())
        };
        let p_count = sym(cf, "CFArrayGetCount");
        let p_at = sym(cf, "CFArrayGetValueAtIndex");
        let p_dict = sym(cf, "CFDictionaryGetValue");
        let p_mkstr = sym(cf, "CFStringCreateWithCString");
        let p_getstr = sym(cf, "CFStringGetCString");
        let p_getnum = sym(cf, "CFNumberGetValue");
        let p_info = sym(iokit, "IOPSCopyPowerSourcesInfo");
        let p_list = sym(iokit, "IOPSCopyPowerSourcesList");
        let p_desc = sym(iokit, "IOPSGetPowerSourceDescription");
        if p_count.is_null() || p_at.is_null() || p_dict.is_null() || p_mkstr.is_null() || p_getstr.is_null() || p_getnum.is_null() || p_info.is_null() || p_list.is_null() || p_desc.is_null() {
            return None;
        }
        type FnInfo = unsafe extern "C" fn() -> *mut std::ffi::c_void;
        type FnList = unsafe extern "C" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void;
        type FnDesc = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        type FnCount = unsafe extern "C" fn(*mut std::ffi::c_void) -> i64;
        type FnAt = unsafe extern "C" fn(*mut std::ffi::c_void, i64) -> *mut std::ffi::c_void;
        type FnDict = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        type FnMk = unsafe extern "C" fn(*mut std::ffi::c_void, *const i8, u32) -> *mut std::ffi::c_void;
        type FnGetS = unsafe extern "C" fn(*mut std::ffi::c_void, *mut i8, i64, u32) -> bool;
        type FnGetN = unsafe extern "C" fn(*mut std::ffi::c_void, i32, *mut i64) -> bool;
        let f_info: FnInfo = std::mem::transmute(p_info);
        let f_list: FnList = std::mem::transmute(p_list);
        let f_desc: FnDesc = std::mem::transmute(p_desc);
        let f_count: FnCount = std::mem::transmute(p_count);
        let f_at: FnAt = std::mem::transmute(p_at);
        let f_dict: FnDict = std::mem::transmute(p_dict);
        let f_mk: FnMk = std::mem::transmute(p_mkstr);
        let f_gets: FnGetS = std::mem::transmute(p_getstr);
        let f_getn: FnGetN = std::mem::transmute(p_getnum);
        let info = f_info();
        if info.is_null() {
            return None;
        }
        let sources = f_list(info);
        if sources.is_null() || f_count(sources) < 1 {
            return None;
        }
        let desc = f_desc(info, f_at(sources, 0));
        if desc.is_null() {
            return None;
        }
        let get_val = |key: &[u8]| -> Option<(Option<String>, Option<i64>)> {
            let k = f_mk(std::ptr::null_mut(), key.as_ptr() as *const i8, 0x08000100);
            if k.is_null() {
                return None;
            }
            let item = f_dict(desc, k);
            if item.is_null() {
                return None;
            }
            let mut buf = [0i8; 32];
            if f_gets(item, buf.as_mut_ptr(), 32, 0x08000100) {
                let mut len = 0usize;
                while len < 32 && buf[len] != 0 {
                    len += 1;
                }
                let bytes: Vec<u8> = buf[..len].iter().map(|&c| c as u8).collect();
                if let Ok(s) = String::from_utf8(bytes) {
                    return Some((Some(s), None));
                }
            }
            let mut num: i64 = 0;
            if f_getn(item, 4, &mut num) {
                return Some((None, Some(num)));
            }
            None
        };
        let cap = get_val(b"Current Capacity\0")?.1?;
        if !(0..=100).contains(&cap) {
            return None;
        }
        let state_raw = get_val(b"Power Source State\0")?.0.unwrap_or("unknown".to_string());
        let state = match state_raw.as_str() {
            "AC Power" => "charging",
            "Battery Power" => "discharging",
            _ => "unknown",
        }
        .to_string();
        let seconds = if state_raw == "AC Power" {
            None
        } else {
            get_val(b"Time to Empty\0").and_then(|(_, n)| n)
        };
        Some(Battery {
            percent: Some(cap),
            state: Some(state),
            seconds_left: seconds,
        })
    }
}

// ---------------------------------------------------------------------------
// Linux
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
fn linux_meminfo() -> HashMap<String, i64> {
    let mut m = HashMap::new();
    if let Ok(text) = fs::read_to_string("/proc/meminfo") {
        for line in text.lines() {
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_string();
                let rest = line[colon + 1..].trim();
                if let Some(num) = rest.split_whitespace().next() {
                    if let Ok(v) = num.parse::<i64>() {
                        m.insert(key, v * 1024);
                    }
                }
            }
        }
    }
    m
}

#[cfg(target_os = "linux")]
fn linux_memory_and_swap(
    mem: &HashMap<String, i64>,
) -> (Option<f64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let total = mem.get("MemTotal").cloned();
    let avail = mem.get("MemAvailable").cloned();
    let (pct, used) = match (total, avail) {
        (Some(t), Some(a)) if t != 0 => {
            let u = (t - a).max(0);
            let p = ((u as f64) * 100.0 / (t as f64) * 10.0).round() / 10.0;
            (Some(p), Some(u))
        }
        _ => {
            let u = match (total, avail) {
                (Some(t), Some(a)) => Some((t - a).max(0)),
                _ => None,
            };
            (None, u)
        }
    };
    let swap_total = mem.get("SwapTotal").cloned();
    let swap = match (swap_total, mem.get("SwapFree").cloned()) {
        (Some(st), Some(sf)) => Some((st - sf).max(0)),
        _ => None,
    };
    (pct, total, used, swap, swap_total)
}

#[cfg(target_os = "linux")]
fn linux_psi() -> Option<HashMap<String, HashMap<String, f64>>> {
    let mut out: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for resource in ["cpu", "memory", "io"] {
        let path = format!("/proc/pressure/{}", resource);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut rows: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let mut m = HashMap::new();
            for item in &parts[1..] {
                if let Some(eq) = item.find('=') {
                    if let Ok(v) = item[eq + 1..].parse::<f64>() {
                        m.insert(item[..eq].to_string(), v);
                    }
                }
            }
            rows.insert(parts[0].to_string(), m);
        }
        if rows.is_empty() {
            continue;
        }
        let mut entry = HashMap::new();
        for stall in ["some", "full"] {
            for window in ["avg10", "avg60", "avg300"] {
                if let Some(r) = rows.get(stall) {
                    if let Some(v) = r.get(window) {
                        entry.insert(format!("{}_{}", stall, window), *v);
                    }
                }
            }
        }
        if !entry.is_empty() {
            out.insert(resource.to_string(), entry);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(target_os = "linux")]
fn linux_pressure(mem: &HashMap<String, i64>) -> (String, String) {
    match fs::read_to_string("/proc/pressure/memory") {
        Ok(text) => {
            let mut some: Option<f64> = None;
            let mut full: Option<f64> = None;
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                if parts[0] == "some" {
                    for item in &parts[1..] {
                        if item.starts_with("avg60=") {
                            some = item[6..].parse::<f64>().ok();
                        }
                    }
                } else if parts[0] == "full" {
                    for item in &parts[1..] {
                        if item.starts_with("avg60=") {
                            full = item[6..].parse::<f64>().ok();
                        }
                    }
                }
            }
            match (some, full) {
                (Some(s), Some(f)) => {
                    let level = if f >= 5.0 {
                        "critical"
                    } else if s >= 15.0 {
                        "warning"
                    } else {
                        "normal"
                    };
                    (level.to_string(), "psi-avg60".to_string())
                }
                _ => ("unknown".to_string(), "psi-avg60".to_string()),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let total = mem.get("MemTotal").cloned();
            let avail = mem.get("MemAvailable").cloned();
            match (total, avail) {
                (Some(t), Some(a)) if t > 0 => {
                    let ratio = (a as f64) / (t as f64);
                    let level = if ratio <= 0.05 {
                        "critical"
                    } else if ratio <= 0.10 {
                        "warning"
                    } else {
                        "normal"
                    };
                    (level.to_string(), "memavailable".to_string())
                }
                _ => ("unknown".to_string(), "memavailable".to_string()),
            }
        }
        Err(_) => ("unknown".to_string(), "psi-avg60".to_string()),
    }
}

#[cfg(target_os = "linux")]
fn linux_thermal() -> Option<String> {
    let mut hottest: Option<f64> = None;
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.starts_with("thermal_zone") {
                continue;
            }
            let temp_path = p.join("temp");
            if let Ok(t) = fs::read_to_string(&temp_path) {
                if let Ok(v) = t.trim().parse::<f64>() {
                    let c = v / 1000.0;
                    hottest = Some(hottest.map_or(c, |h: f64| h.max(c)));
                }
            }
        }
    }
    hottest.map(|h| {
        if h >= 95.0 {
            "critical".to_string()
        } else if h >= 85.0 {
            "serious".to_string()
        } else if h >= 75.0 {
            "fair".to_string()
        } else {
            "nominal".to_string()
        }
    })
}

#[cfg(target_os = "linux")]
fn linux_battery() -> Option<Battery> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        let typ = fs::read_to_string(p.join("type")).unwrap_or_default();
        if typ.trim() != "Battery" {
            continue;
        }
        let cap = fs::read_to_string(p.join("capacity"))
            .ok()?
            .trim()
            .parse::<i64>()
            .ok()?;
        let status = fs::read_to_string(p.join("status")).unwrap_or_default();
        let state = match status.trim().to_lowercase().as_str() {
            "charging" => "charging",
            "full" => "full",
            "discharging" => "discharging",
            _ => "unknown",
        }
        .to_string();
        return Some(Battery {
            percent: Some(cap),
            state: Some(state),
            seconds_left: None,
        });
    }
    None
}

// ---------------------------------------------------------------------------
// Disks
// ---------------------------------------------------------------------------
fn default_mounts() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let drive = std::env::var("SystemDrive").unwrap_or("C:".to_string());
        vec![format!("{}\\", drive.trim_end_matches('\\'))]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec!["/".to_string()]
    }
}

#[cfg(unix)]
fn unix_disks(mounts: &[String]) -> Vec<Disk> {
    let mut out = Vec::new();
    for mount in mounts {
        unsafe {
            let c = match std::ffi::CString::new(mount.as_str()) {
                Ok(v) => v,
                Err(_) => {
                    out.push(Disk {
                        mount: mount.clone(),
                        total_bytes: None,
                        used_bytes: None,
                        free_bytes: None,
                        percent: None,
                    });
                    continue;
                }
            };
            let mut st: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c.as_ptr(), &mut st) != 0 {
                out.push(Disk {
                    mount: mount.clone(),
                    total_bytes: None,
                    used_bytes: None,
                    free_bytes: None,
                    percent: None,
                });
                continue;
            }
            // Match shutil.disk_usage: total=blocks*frsize, used=(blocks-bfree)*frsize, free=bavail*frsize.
            let frsize = st.f_frsize as u64;
            let blocks = st.f_blocks as u64;
            let bfree = st.f_bfree as u64;
            let bavail = st.f_bavail as u64;
            let total = blocks.saturating_mul(frsize);
            let used = blocks.saturating_sub(bfree).saturating_mul(frsize);
            let free_b = bavail.saturating_mul(frsize);
            let pct = if total == 0 {
                None
            } else {
                Some(((used as f64) * 100.0 / (total as f64) * 10.0).round() / 10.0)
            };
            out.push(Disk {
                mount: mount.clone(),
                total_bytes: Some(total as i64),
                used_bytes: Some(used as i64),
                free_bytes: Some(free_b as i64),
                percent: pct,
            });
        }
    }
    out
}

fn disks(mounts: Option<Vec<String>>) -> Vec<Disk> {
    #[cfg(target_os = "windows")]
    {
        return crate::win::windows_disks(&mounts.unwrap_or_else(default_mounts));
    }
    #[cfg(unix)]
    {
        let list = mounts.unwrap_or_else(default_mounts);
        let l = if list.is_empty() {
            default_mounts()
        } else {
            list
        };
        unix_disks(&l)
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        mounts
            .unwrap_or_else(default_mounts)
            .into_iter()
            .map(|m| Disk {
                mount: m,
                total_bytes: None,
                used_bytes: None,
                free_bytes: None,
                percent: None,
            })
            .collect()
    }
}

fn loadaverage() -> Option<Vec<f64>> {
    #[cfg(unix)]
    {
        unsafe {
            let mut loads = [0.0f64; 3];
            let r = libc::getloadavg(loads.as_mut_ptr(), 3);
            if r == 3
                && loads.iter().all(|v| v.is_finite())
            {
                return Some(loads.to_vec());
            }
            None
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

// ---------------------------------------------------------------------------
// CPU persisted baseline + sample
// ---------------------------------------------------------------------------
fn state_dir(arg: Option<&str>) -> PathBuf {
    if let Some(d) = arg {
        return PathBuf::from(d);
    }
    if let Some(v) = std::env::var_os("HOST_PRESSURE_STATE_DIR") {
        return PathBuf::from(v);
    }
    let base = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        home.join(".local/state")
    });
    base.join("host-pressure")
}

fn safe_consumer(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug)]
struct Baseline {
    boot_time: Option<f64>,
    ticks: Option<Vec<u64>>,
    valid: bool,
}

fn parse_baseline(text: &str, now: f64) -> Option<(f64, Vec<u64>)> {
    // Tiny parser for {"boot_time": X, "ticks": [...], "ts": Y}
    // boot_time may be null.
    let boot: Option<f64> = {
        if let Some(idx) = text.find("\"boot_time\"") {
            let rest = &text[idx..];
            if let Some(colon) = rest.find(':') {
                let after = rest[colon + 1..].trim_start();
                if after.starts_with("null") {
                    None
                } else {
                    let mut end = 0usize;
                    for (k, ch) in after.char_indices() {
                        if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' || ch == 'e' || ch == 'E' {
                            end = k + ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    after[..end].parse::<f64>().ok()
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    };
    let ticks: Option<Vec<u64>> = {
        if let Some(idx) = text.find("\"ticks\"") {
            let rest = &text[idx..];
            if let Some(lb) = rest.find('[') {
                let after = &rest[lb + 1..];
                if let Some(rb) = after.find(']') {
                    let inner = &after[..rb].trim();
                    if inner.starts_with("null") {
                        None
                    } else if inner.is_empty() {
                        Some(vec![])
                    } else {
                        let mut v = Vec::new();
                        for part in inner.split(',') {
                            let p = part.trim();
                            if p == "null" {
                                return None;
                            }
                            match p.parse::<u64>() {
                                Ok(n) => v.push(n),
                                Err(_) => return None,
                            }
                        }
                        Some(v)
                    }
                } else {
                    return None;
                }
            } else if rest.contains("null") {
                None
            } else {
                return None;
            }
        } else {
            return None;
        }
    };
    let ticks = ticks?;
    // validate
    if ticks.len() != ticks_expected() {
        return None;
    }
    let max = tick_limit();
    for t in &ticks {
        if *t > max {
            return None;
        }
    }
    match boot {
        Some(b) => {
            if !(b > 0.0 && b <= now) {
                return None;
            }
            Some((b, ticks))
        }
        None => None,
    }
}

fn valid_ticks(ticks: &Option<Vec<u64>>, boot: &Option<f64>, now: f64) -> bool {
    match (ticks, boot) {
        (Some(t), Some(b)) => {
            if !(*b > 0.0 && *b <= now) {
                return false;
            }
            if t.len() != ticks_expected() {
                return false;
            }
            let max = tick_limit();
            t.iter().all(|x| *x <= max)
        }
        _ => false,
    }
}

#[cfg(unix)]
fn lock_and_read_write<F>(dir: &Path, state_name: &str, lock_name: &str, f: F) -> Option<f64>
where
    F: FnOnce(Option<(f64, Vec<u64>)>, f64, Vec<u64>) -> (Option<f64>, String),
{
    use std::os::unix::io::AsRawFd;
    let lock_path = dir.join(lock_name);
    let state_path_fixed = dir.join(state_name);
    // open (create) lock file
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&lock_path)
        .ok()?;
    let fd = lock_file.as_raw_fd();
    unsafe {
        libc::flock(fd, libc::LOCK_EX);
    }
    let result = (|| {
        let boot = boot_time();
        let current = cpu_ticks()?;
        let now = now_unix();
        if !valid_ticks(&Some(current.clone()), &boot, now) {
            return None;
        }
        let b = boot?;
        // read previous (state shares Python's path/format)
        let prev: Option<(f64, Vec<u64>)> = {
            let sp = &state_path_fixed;
            if let Ok(meta) = std::fs::metadata(sp) {
                if meta.len() <= 4096 {
                    if let Ok(text) = std::fs::read_to_string(sp) {
                        parse_baseline(&text, now)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };
        let (cpu, payload) = f(prev.clone(), b, current.clone());
        // write payload atomically
        let sp = &state_path_fixed;
        let tmp = dir.join(format!(".{}.tmp{}", sp.file_name().unwrap().to_string_lossy(), std::process::id()));
        if std::fs::write(&tmp, payload).is_ok() {
            let _ = std::fs::rename(&tmp, &sp);
            let _ = std::fs::remove_file(&tmp);
        }
        cpu
    })();
    unsafe {
        libc::flock(fd, libc::LOCK_UN);
    }
    result
}

pub fn get_persisted_cpu_percent(consumer_id: &str, state_dir_arg: Option<&str>) -> Option<f64> {
    let safe_id = if consumer_id.is_empty() {
        "default".to_string()
    } else {
        safe_consumer(consumer_id)
    };
    let dir = state_dir(state_dir_arg);
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let state_name = format!("cpu_ticks_{}.json", safe_id);
    #[cfg(unix)]
    {
        // Match Python: state cpu_ticks_<id>.json, lock cpu_ticks_<id>.lock (with_suffix).
        let lock_name = format!("cpu_ticks_{}.lock", safe_id);
        lock_and_read_write(&dir, &state_name, &lock_name, |prev, boot, current| {
            let mut cpu: Option<f64> = None;
            if let Some((prev_boot, prev_ticks)) = prev {
                if (prev_boot - boot).abs() < 2.0 {
                    let mut delta: Vec<u64> = Vec::new();
                    for (a, b_) in current.iter().zip(prev_ticks.iter()) {
                        delta.push(a.wrapping_sub(*b_));
                    }
                    #[cfg(target_os = "macos")]
                    {
                        delta = delta.iter().map(|v| v & 0xFFFFFFFF).collect();
                    }
                    // check all forward (wrapping already handled for darwin; for others wrapping_sub could wrap on reboot but boot check guards)
                    // For Linux/others, require a >= b (no wrap).
                    #[cfg(not(target_os = "macos"))]
                    {
                        let mut ok = true;
                        for (a, b_) in current.iter().zip(prev_ticks.iter()) {
                            if *a < *b_ {
                                ok = false;
                                break;
                            }
                        }
                        if !ok {
                            delta = vec![];
                        }
                    }
                    if !delta.is_empty() {
                        cpu = busy_percent(&delta);
                    }
                }
            }
            let payload = format!(
                "{{\"boot_time\": {}, \"ticks\": [{}], \"ts\": {}}}",
                boot,
                current.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "),
                now_unix()
            );
            (cpu, payload)
        })
    }
    #[cfg(not(unix))]
    {
        // Windows: atomic replace without flock (same-dir rename is atomic).
        let boot = boot_time()?;
        let current = cpu_ticks()?;
        let now = now_unix();
        if !valid_ticks(&Some(current.clone()), &Some(boot), now) {
            return None;
        }
        let state_path = dir.join(&state_name);
        let mut cpu: Option<f64> = None;
        if let Ok(meta) = fs::metadata(&state_path) {
            if meta.len() <= 4096 {
                if let Ok(text) = fs::read_to_string(&state_path) {
                    if let Some((prev_boot, prev_ticks)) = parse_baseline(&text, now) {
                        if (prev_boot - boot).abs() < 2.0 {
                            let delta: Vec<u64> = current.iter().zip(prev_ticks.iter()).map(|(a, b)| a.saturating_sub(*b)).collect();
                            cpu = busy_percent(&delta);
                        }
                    }
                }
            }
        }
        let payload = format!(
            "{{\"boot_time\": {}, \"ticks\": [{}], \"ts\": {}}}",
            boot,
            current.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "),
            now
        );
        let tmp = dir.join(format!(".{}.tmp{}", state_name, std::process::id()));
        if fs::write(&tmp, payload).is_ok() {
            let _ = fs::rename(&tmp, &state_path);
            let _ = fs::remove_file(&tmp);
        }
        cpu
    }
}

pub fn sample_cpu_percent(seconds: f64) -> Option<f64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let window = seconds.max(0.05).min(10.0);
    let before = cpu_ticks()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(window + 1.0);
    std::thread::sleep(std::time::Duration::from_secs_f64(window));
    loop {
        if let Some(after) = cpu_ticks() {
            if after.len() == before.len() {
                let mut delta: Vec<u64> = Vec::new();
                let mut ok = true;
                for (a, b) in after.iter().zip(before.iter()) {
                    #[cfg(target_os = "macos")]
                    {
                        delta.push(a.wrapping_sub(*b) & 0xFFFFFFFF);
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        if *a < *b {
                            ok = false;
                            break;
                        }
                        delta.push(*a - *b);
                    }
                }
                if ok && delta.iter().sum::<u64>() > 0 {
                    if idle_ticks(&delta) > 0 || std::time::Instant::now() >= deadline {
                        return busy_percent(&delta);
                    }
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

// ---------------------------------------------------------------------------
// Public reading
// ---------------------------------------------------------------------------
pub fn read_host_resources(
    consumer_id: &str,
    state_dir_arg: Option<&str>,
    mounts: Option<Vec<String>>,
    sample: Option<f64>,
) -> Reading {
    read_resources(consumer_id, state_dir_arg, mounts, sample, true)
}

/// Status collectors do not need thermal/battery objects or their platform frameworks.
/// The existing CLI keeps the full reading above, with identical output.
pub fn read_host_metrics(consumer_id: &str, state_dir_arg: Option<&str>) -> Reading {
    read_resources(consumer_id, state_dir_arg, None, None, false)
}

fn read_resources(
    consumer_id: &str,
    state_dir_arg: Option<&str>,
    mounts: Option<Vec<String>>,
    sample: Option<f64>,
    extras: bool,
) -> Reading {
    let now_str = iso_now();
    let os_s = current_os();
    let host = hostname();
    let ncpu_v = ncpu();

    let mut memory_percent: Option<f64> = None;
    let mut memory_total: Option<i64> = None;
    let mut memory_used: Option<i64> = None;
    let mut swap_bytes: Option<i64> = None;
    let mut swap_total: Option<i64> = None;
    let mut pressure = "unknown".to_string();
    let mut pressure_source = "unknown".to_string();
    let mut thermal: Option<String> = None;
    let mut battery: Option<Battery> = None;
    let mut psi: Option<HashMap<String, HashMap<String, f64>>> = None;

    #[cfg(target_os = "macos")]
    {
        let (p, t, u) = darwin_memory();
        memory_percent = p;
        memory_total = t;
        memory_used = u;
        let (pr, prs) = darwin_pressure();
        pressure = pr;
        pressure_source = prs;
        let (sb, st) = darwin_swap();
        swap_bytes = sb;
        swap_total = st;
        if extras {
            thermal = darwin_thermal();
            battery = darwin_battery();
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mem = linux_meminfo();
        let (p, t, u, sb, st) = linux_memory_and_swap(&mem);
        memory_percent = p;
        memory_total = t;
        memory_used = u;
        swap_bytes = sb;
        swap_total = st;
        let (pr, prs) = linux_pressure(&mem);
        pressure = pr;
        pressure_source = prs;
        if extras {
            psi = linux_psi();
            thermal = linux_thermal();
            battery = linux_battery();
        }
    }
    #[cfg(target_os = "windows")]
    {
        let (p, t, u, commit) = crate::win::windows_memory();
        memory_percent = p;
        memory_total = t;
        memory_used = u;
        let (sb, st) = crate::win::windows_pagefile();
        swap_bytes = sb;
        swap_total = st;
        let _ = commit;
        if let Some(mp) = memory_percent {
            if mp >= 95.0 {
                pressure = "critical".to_string();
            } else if mp >= 85.0 {
                pressure = "warning".to_string();
            } else {
                pressure = "normal".to_string();
            }
            pressure_source = "memoryload".to_string();
        }
        battery = crate::win::windows_battery();
    }

    let (cpu_percent, cpu_source) = match sample {
        Some(s) => {
            let v = sample_cpu_percent(s);
            (v, v.map(|_| "sample".to_string()))
        }
        None => {
            let v = get_persisted_cpu_percent(consumer_id, state_dir_arg);
            let src = v.map(|_| "baseline".to_string());
            (v, src)
        }
    };

    let up_secs = uptime_seconds();
    let up_str = format_uptime(up_secs);
    let up_rounded = up_secs.map(|v| (v * 10.0).round() / 10.0);

    Reading {
        observed_at: Some(now_str),
        os: Some(os_s),
        hostname: host,
        ncpu: ncpu_v,
        cpu_percent,
        cpu_source,
        memory_percent,
        memory_total_bytes: memory_total,
        memory_used_bytes: memory_used,
        swap_bytes,
        swap_total_bytes: swap_total,
        pressure,
        pressure_source,
        loadavg: loadaverage(),
        uptime: up_str,
        uptime_seconds: up_rounded,
        thermal,
        battery,
        psi,
        disks: disks(mounts),
    }
}
