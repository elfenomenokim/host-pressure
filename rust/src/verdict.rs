use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub const LEVELS: [&str; 3] = ["ok", "busy", "overloaded"];

pub const NUMERIC_KEYS: [&str; 16] = [
    "cpu_busy",
    "cpu_overloaded",
    "memory_busy",
    "memory_overloaded",
    "load_busy",
    "load_overloaded",
    "disk_busy",
    "disk_overloaded",
    "swap_busy_gb",
    "swap_overloaded_gb",
    "psi_cpu_busy",
    "psi_cpu_overloaded",
    "psi_memory_busy",
    "psi_memory_overloaded",
    "psi_io_busy",
    "psi_io_overloaded",
];

#[derive(Debug, Clone)]
pub struct Thresholds {
    pub cpu_busy: f64,
    pub cpu_overloaded: f64,
    pub memory_busy: f64,
    pub memory_overloaded: f64,
    pub load_busy: f64,
    pub load_overloaded: f64,
    pub disk_busy: f64,
    pub disk_overloaded: f64,
    pub swap_busy_gb: Option<f64>,
    pub swap_overloaded_gb: Option<f64>,
    pub psi_cpu_busy: f64,
    pub psi_cpu_overloaded: f64,
    pub psi_memory_busy: f64,
    pub psi_memory_overloaded: f64,
    pub psi_io_busy: f64,
    pub psi_io_overloaded: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu_busy: 80.0,
            cpu_overloaded: 95.0,
            memory_busy: 85.0,
            memory_overloaded: 95.0,
            load_busy: 1.0,
            load_overloaded: 2.0,
            disk_busy: 90.0,
            disk_overloaded: 97.0,
            swap_busy_gb: None,
            swap_overloaded_gb: None,
            psi_cpu_busy: 50.0,
            psi_cpu_overloaded: 90.0,
            psi_memory_busy: 15.0,
            psi_memory_overloaded: 5.0,
            psi_io_busy: 50.0,
            psi_io_overloaded: 90.0,
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Some(p) = crate::core::env_path("HOST_PRESSURE_CONFIG") {
        return p;
    }
    let base = crate::core::env_path("XDG_CONFIG_HOME")
        .unwrap_or_else(|| crate::core::home_dir().join(".config"));
    base.join("host-pressure").join("config.json")
}

/// Minimal JSON object parser for the flat threshold config.
/// Values are numbers or null; unknown keys are reported.
fn parse_flat_object(text: &str) -> Result<HashMap<String, Option<f64>>, String> {
    let mut map = HashMap::new();
    let t = text.trim();
    if t.is_empty() {
        return Err("expected a JSON object".to_string());
    }
    if !t.starts_with('{') {
        return Err("expected a JSON object".to_string());
    }
    // Find matching end brace (config is flat; no nesting expected except error).
    // Do a tiny hand parse: strings, numbers, null, commas, colons.
    let bytes = t.as_bytes();
    let mut i = 0usize;
    let n = bytes.len();
    let skip_ws = |i: &mut usize| {
        while *i < n && (bytes[*i] == b' ' || bytes[*i] == b'\t' || bytes[*i] == b'\n' || bytes[*i] == b'\r') {
            *i += 1;
        }
    };
    i += 1; // {
    skip_ws(&mut i);
    if i < n && bytes[i] == b'}' {
        return Ok(map);
    }
    loop {
        skip_ws(&mut i);
        if i >= n || bytes[i] != b'"' {
            return Err("expected a JSON object".to_string());
        }
        // parse string key
        i += 1;
        let mut key = String::new();
        let mut esc = false;
        let mut closed = false;
        while i < n {
            let c = bytes[i];
            if esc {
                match c {
                    b'"' => key.push('"'),
                    b'\\' => key.push('\\'),
                    b'n' => key.push('\n'),
                    b't' => key.push('\t'),
                    b'r' => key.push('\r'),
                    _ => key.push(c as char),
                }
                esc = false;
                i += 1;
            } else if c == b'\\' {
                esc = true;
                i += 1;
            } else if c == b'"' {
                closed = true;
                i += 1;
                break;
            } else {
                key.push(c as char);
                i += 1;
            }
        }
        if !closed {
            return Err("expected a JSON object".to_string());
        }
        skip_ws(&mut i);
        if i >= n || bytes[i] != b':' {
            return Err("expected a JSON object".to_string());
        }
        i += 1;
        skip_ws(&mut i);
        if i >= n {
            return Err("expected a JSON object".to_string());
        }
        // value: null or number
        if t[i..].starts_with("null") {
            map.insert(key, None);
            i += 4;
        } else {
            let start = i;
            if i < n && (bytes[i] == b'-' || bytes[i] == b'+') {
                i += 1;
            }
            while i < n && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E' || bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            let num_s = &t[start..i];
            if num_s.is_empty() {
                return Err("expected a JSON object".to_string());
            }
            match num_s.parse::<f64>() {
                Ok(v) => {
                    map.insert(key, Some(v));
                }
                Err(_) => return Err("expected a JSON object".to_string()),
            }
        }
        skip_ws(&mut i);
        if i >= n {
            return Err("expected a JSON object".to_string());
        }
        if bytes[i] == b',' {
            i += 1;
            continue;
        } else if bytes[i] == b'}' {
            i += 1;
            skip_ws(&mut i);
            break;
        } else {
            return Err("expected a JSON object".to_string());
        }
    }
    return Ok(map);
}

pub fn read_config(path: Option<&str>) -> Result<HashMap<String, Option<f64>>, String> {
    let target: PathBuf = match path {
        Some(p) => PathBuf::from(p),
        None => config_path(),
    };
    let text = match std::fs::read_to_string(&target) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(e) => return Err(format!("{}: {}", target.display(), e)),
    };
    if text.trim().is_empty() {
        return Err(format!("{}: expected a JSON object", target.display()));
    }
    let map = parse_flat_object(&text).map_err(|_| format!("{}: expected a JSON object", target.display()))?;
    // Unknown keys check needs original structure; re-derive unknown from map keys.
    let unknown: Vec<String> = {
        let mut u: Vec<String> = map
            .keys()
            .filter(|k| !NUMERIC_KEYS.contains(&k.as_str()))
            .cloned()
            .collect();
        u.sort();
        u
    };
    if !unknown.is_empty() {
        return Err(format!(
            "{}: unknown key(s) {}; valid keys: {}",
            target.display(),
            unknown.join(", "),
            NUMERIC_KEYS.join(", ")
        ));
    }
    // A null threshold is meaningful only for the swap limits; elsewhere it
    // used to be silently ignored while Python crashed on the comparison.
    for (key, value) in &map {
        if value.is_none() && !key.starts_with("swap_") {
            return Err(format!(
                "{}: {}: only swap_busy_gb and swap_overloaded_gb accept off, none or null",
                target.display(),
                key
            ));
        }
    }
    // Detect non-object JSON (e.g. array): our tiny parser already errors as expected-object.
    // Also detect trailing garbage after object.
    Ok(map)
}

pub fn load_thresholds(
    path: Option<&str>,
    overrides: &HashMap<String, Option<f64>>,
) -> Result<Thresholds, String> {
    let mut values: HashMap<String, Option<f64>> = HashMap::new();
    let cfg = read_config(path)?;
    for (k, v) in cfg {
        values.insert(k, v);
    }
    for key in NUMERIC_KEYS {
        let env_name = format!("HOST_PRESSURE_{}", key.to_uppercase());
        if let Some(raw_os) = env::var_os(&env_name) {
            let raw = raw_os.to_string_lossy().into_owned();
            if raw.is_empty() {
                continue;
            }
            let low = raw.to_lowercase();
            if low == "none" || low == "off" || low == "null" {
                if !key.starts_with("swap_") {
                    return Err(format!(
                        "{}: only swap_busy_gb and swap_overloaded_gb accept off, none or null",
                        env_name
                    ));
                }
                values.insert(key.to_string(), None);
                continue;
            }
            match raw.parse::<f64>() {
                Ok(v) => {
                    values.insert(key.to_string(), Some(v));
                }
                Err(_) => {
                    return Err(format!("{}: expected a number, got '{}'", env_name, raw));
                }
            }
        }
    }
    for (k, v) in overrides {
        if let Some(val) = v {
            values.insert(k.clone(), Some(*val));
        }
    }
    if values.is_empty() {
        return Ok(Thresholds::default());
    }
    let mut t = Thresholds::default();
    for (k, v) in values {
        match k.as_str() {
            "cpu_busy" => if let Some(x) = v { t.cpu_busy = x; },
            "cpu_overloaded" => if let Some(x) = v { t.cpu_overloaded = x; },
            "memory_busy" => if let Some(x) = v { t.memory_busy = x; },
            "memory_overloaded" => if let Some(x) = v { t.memory_overloaded = x; },
            "load_busy" => if let Some(x) = v { t.load_busy = x; },
            "load_overloaded" => if let Some(x) = v { t.load_overloaded = x; },
            "disk_busy" => if let Some(x) = v { t.disk_busy = x; },
            "disk_overloaded" => if let Some(x) = v { t.disk_overloaded = x; },
            "swap_busy_gb" => t.swap_busy_gb = v,
            "swap_overloaded_gb" => t.swap_overloaded_gb = v,
            "psi_cpu_busy" => if let Some(x) = v { t.psi_cpu_busy = x; },
            "psi_cpu_overloaded" => if let Some(x) = v { t.psi_cpu_overloaded = x; },
            "psi_memory_busy" => if let Some(x) = v { t.psi_memory_busy = x; },
            "psi_memory_overloaded" => if let Some(x) = v { t.psi_memory_overloaded = x; },
            "psi_io_busy" => if let Some(x) = v { t.psi_io_busy = x; },
            "psi_io_overloaded" => if let Some(x) = v { t.psi_io_overloaded = x; },
            _ => {}
        }
    }
    Ok(t)
}

/// Python-compatible "%0.1f then strip" number.
pub fn py_num(value: f64) -> String {
    if !value.is_finite() {
        return format!("{}", value);
    }
    let s = format!("{:.1}", value);
    let mut t = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if t.is_empty() || t == "-0" {
        t = "0".to_string();
    }
    t
}

struct Found {
    level: String,
    reason: String,
}

fn above(
    value: Option<f64>,
    busy: f64,
    overloaded: f64,
    label: &str,
    unit: &str,
) -> Option<Found> {
    let v = value?;
    if v >= overloaded {
        Some(Found {
            level: "overloaded".to_string(),
            reason: format!("{} {} >= {}", label, py_num(v) + unit, py_num(overloaded) + unit),
        })
    } else if v >= busy {
        Some(Found {
            level: "busy".to_string(),
            reason: format!("{} {} >= {}", label, py_num(v) + unit, py_num(busy) + unit),
        })
    } else {
        None
    }
}

fn severity(level: &str) -> i32 {
    match level {
        "ok" => 0,
        "busy" => 1,
        "overloaded" => 2,
        _ => -1,
    }
}

#[derive(Debug, Clone)]
pub struct Verdict {
    pub level: String,
    pub reasons: Vec<String>,
}

pub fn at_least(level: &str) -> Result<i32, String> {
    match level {
        "ok" | "busy" | "overloaded" => Ok(severity(level)),
        _ => Err(format!(
            "unknown level '{}'; expected one of {}",
            level,
            LEVELS.join(", ")
        )),
    }
}

pub fn verdict_reading(r: &crate::core::Reading, rules: &Thresholds) -> Verdict {
    let mut found: Vec<Found> = Vec::new();

    if let Some(f) = above(r.cpu_percent, rules.cpu_busy, rules.cpu_overloaded, "cpu", "%") {
        found.push(f);
    }
    if let Some(f) = above(
        r.memory_percent,
        rules.memory_busy,
        rules.memory_overloaded,
        "memory",
        "%",
    ) {
        found.push(f);
    }

    if let Some(load) = r.loadavg.clone() {
        let cores = r.ncpu.unwrap_or(1) as f64;
        if load.len() == 3 && cores != 0.0 {
            let per_core = load[0] / cores;
            let label = format!("load {} ({:.2}/core)", py_num(load[0]), per_core);
            if let Some(f) = above(
                Some(per_core),
                rules.load_busy,
                rules.load_overloaded,
                &label,
                "/core",
            ) {
                found.push(f);
            }
        }
    }

    for d in &r.disks {
        if let Some(p) = d.percent {
            if let Some(f) = above(
                Some(p),
                rules.disk_busy,
                rules.disk_overloaded,
                &format!("disk {}", d.mount),
                "%",
            ) {
                found.push(f);
            }
        }
    }

    // On Linux the pressure label is a fixed PSI mapping. The configurable
    // psi_* rules above already read those same numbers, so applying the label
    // again would override a raised threshold. The macOS pressure mapping and
    // the memory-available fallback keep their fixed meaning.
    if r.pressure_source != "psi-avg60" {
        match r.pressure.as_str() {
            "critical" => found.push(Found {
                level: "overloaded".to_string(),
                reason: "memory pressure critical".to_string(),
            }),
            "warning" => found.push(Found {
                level: "busy".to_string(),
                reason: "memory pressure warning".to_string(),
            }),
            _ => {}
        }
    }

    match r.thermal.as_deref() {
        Some("critical") | Some("serious") => found.push(Found {
            level: "overloaded".to_string(),
            reason: format!("thermal {}", r.thermal.clone().unwrap()),
        }),
        Some("fair") => found.push(Found {
            level: "busy".to_string(),
            reason: "thermal fair".to_string(),
        }),
        _ => {}
    }

    if let Some(swap_bytes) = r.swap_bytes {
        if rules.swap_busy_gb.is_some() || rules.swap_overloaded_gb.is_some() {
            let used = swap_bytes as f64 / 1024.0_f64.powi(3);
            let busy = rules.swap_busy_gb.unwrap_or(f64::INFINITY);
            let overloaded = rules.swap_overloaded_gb.unwrap_or(f64::INFINITY);
            if let Some(f) = above(Some(used), busy, overloaded, "swap", "G") {
                found.push(f);
            }
        }
    }

    if let Some(psi) = r.psi.clone() {
        if let Some(mem) = psi.get("memory") {
            if let Some(v) = mem.get("some_avg60") {
                if let Some(f) = above(
                    Some(*v),
                    rules.psi_memory_busy,
                    f64::INFINITY,
                    "psi memory some avg60",
                    "",
                ) {
                    found.push(f);
                }
            }
            if let Some(full) = mem.get("full_avg60") {
                if *full >= rules.psi_memory_overloaded {
                    found.push(Found {
                        level: "overloaded".to_string(),
                        reason: format!(
                            "psi memory full avg60 {} >= {}",
                            py_num(*full),
                            py_num(rules.psi_memory_overloaded)
                        ),
                    });
                }
            }
        }
        for resource in ["cpu", "io"] {
            let (busy, overloaded) = if resource == "cpu" {
                (rules.psi_cpu_busy, rules.psi_cpu_overloaded)
            } else {
                (rules.psi_io_busy, rules.psi_io_overloaded)
            };
            if let Some(series) = psi.get(resource) {
                if let Some(v) = series.get("some_avg60") {
                    if let Some(f) = above(
                        Some(*v),
                        busy,
                        overloaded,
                        &format!("psi {} some avg60", resource),
                        "",
                    ) {
                        found.push(f);
                    }
                }
            }
        }
    }

    let mut level = "ok".to_string();
    for f in &found {
        if severity(&f.level) > severity(&level) {
            level = f.level.clone();
        }
    }
    let reasons: Vec<String> = found
        .iter()
        .filter(|f| severity(&f.level) == severity(&level))
        .map(|f| f.reason.clone())
        .collect();
    Verdict { level, reasons }
}

#[allow(dead_code)]
pub fn config_path_for_test() -> PathBuf {
    Path::new("/nonexistent").to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Reading;

    fn psi_reading() -> Reading {
        let mut memory = HashMap::new();
        memory.insert("some_avg60".to_string(), 20.0);
        memory.insert("full_avg60".to_string(), 6.0);
        let mut psi = HashMap::new();
        psi.insert("memory".to_string(), memory);
        Reading {
            observed_at: None,
            os: Some("linux".to_string()),
            hostname: None,
            ncpu: None,
            cpu_percent: None,
            cpu_source: None,
            memory_percent: None,
            memory_total_bytes: None,
            memory_used_bytes: None,
            swap_bytes: None,
            swap_total_bytes: None,
            pressure: "critical".to_string(),
            pressure_source: "psi-avg60".to_string(),
            loadavg: None,
            uptime: None,
            uptime_seconds: None,
            thermal: None,
            battery: None,
            psi: Some(psi),
            disks: vec![],
        }
    }

    fn relaxed() -> Thresholds {
        Thresholds {
            psi_memory_busy: 50.0,
            psi_memory_overloaded: 50.0,
            ..Thresholds::default()
        }
    }

    #[test]
    fn configured_psi_thresholds_decide_the_linux_verdict() {
        assert_eq!(
            verdict_reading(&psi_reading(), &Thresholds::default()).level,
            "overloaded"
        );
        assert_eq!(verdict_reading(&psi_reading(), &relaxed()).level, "ok");
    }

    #[test]
    fn macos_pressure_label_keeps_its_fixed_mapping() {
        let mut r = psi_reading();
        r.os = Some("darwin".to_string());
        r.pressure_source = "nspressuredispatch".to_string();
        r.psi = None;
        let v = verdict_reading(&r, &relaxed());
        assert_eq!(v.level, "overloaded");
        assert_eq!(v.reasons, vec!["memory pressure critical".to_string()]);
    }

    fn write_config(body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hp-config-test-{}-{}.json",
            std::process::id(),
            body.len()
        ));
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn null_threshold_is_only_for_swap() {
        let path = write_config("{\"memory_busy\": null}");
        let err = read_config(path.to_str()).unwrap_err();
        std::fs::remove_file(&path).ok();
        assert!(err.contains("only swap_busy_gb and swap_overloaded_gb"), "{err}");
    }

    #[test]
    fn null_swap_threshold_and_numbers_are_accepted() {
        let path = write_config("{\"swap_busy_gb\": null, \"memory_busy\": 70}");
        let map = read_config(path.to_str()).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(map.get("swap_busy_gb"), Some(&None));
        assert_eq!(map.get("memory_busy"), Some(&Some(70.0)));
    }

    #[test]
    fn reason_number_formatting_matches_python() {
        let mut r = psi_reading();
        r.os = Some("darwin".to_string());
        r.pressure = "normal".to_string();
        r.pressure_source = "nspressuredispatch".to_string();
        r.psi = None;
        r.ncpu = Some(5);
        r.loadavg = Some(vec![15.0, 15.0, 15.0]);
        let integral = verdict_reading(&r, &Thresholds::default());
        assert_eq!(integral.level, "overloaded");
        assert_eq!(
            integral.reasons,
            vec!["load 15 (3.00/core) 3/core >= 2/core".to_string()]
        );

        r.loadavg = Some(vec![16.3, 16.3, 16.3]);
        let fractional = verdict_reading(&r, &Thresholds::default());
        assert_eq!(fractional.level, "overloaded");
        assert_eq!(
            fractional.reasons,
            vec!["load 16.3 (3.26/core) 3.3/core >= 2/core".to_string()]
        );

        r.loadavg = None;
        r.memory_percent = Some(-0.0);
        let zero_mem = verdict_reading(
            &r,
            &Thresholds {
                memory_busy: 0.0,
                ..Thresholds::default()
            },
        );
        assert_eq!(zero_mem.level, "busy");
        assert_eq!(zero_mem.reasons, vec!["memory 0% >= 0%".to_string()]);
    }
}
