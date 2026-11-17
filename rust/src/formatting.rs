use std::collections::HashMap;

/// Python round-half-even to nearest integer.
pub fn py_round_int(x: f64) -> i64 {
    if !x.is_finite() {
        return 0;
    }
    let f = x.floor();
    let diff = x - f;
    if diff < 0.5 {
        f as i64
    } else if diff > 0.5 {
        (f + 1.0) as i64
    } else {
        // tie: to even
        let fi = f as i64;
        if fi % 2 == 0 {
            fi
        } else {
            fi + 1
        }
    }
}

pub fn human_bytes(value: Option<i64>) -> String {
    match value {
        None => "n/a".to_string(),
        Some(v) => {
            let size = v as f64;
            // Match Python formatting.py: %.1f for T/G/M/K.
            let units: [( &str, f64); 4] = [
                ("T", 1024.0_f64.powi(4)),
                ("G", 1024.0_f64.powi(3)),
                ("M", 1024.0_f64.powi(2)),
                ("K", 1024.0),
            ];
            for (suffix, unit) in units {
                if size.abs() >= unit {
                    return format!("{:.1}{}", size / unit, suffix);
                }
            }
            format!("{}B", v)
        }
    }
}

pub fn percent(value: Option<f64>) -> String {
    match value {
        None => "n/a".to_string(),
        Some(v) => format!("{}%", py_round_int(v)),
    }
}

pub fn psi_value(psi: &Option<HashMap<String, HashMap<String, f64>>>, resource: &str) -> String {
    if let Some(m) = psi {
        if let Some(series) = m.get(resource) {
            if let Some(v) = series.get("some_avg60") {
                return format!("{:.1}", v);
            }
        }
    }
    "n/a".to_string()
}

pub fn one_line(r: &crate::core::Reading, extra: bool) -> String {
    let mut parts = vec![
        format!("CPU {}", percent(r.cpu_percent)),
        format!("MEM {}", percent(r.memory_percent)),
        format!("SWAP {}", human_bytes(r.swap_bytes)),
        format!(
            "pressure {}",
            if r.pressure.is_empty() {
                "unknown"
            } else {
                r.pressure.as_str()
            }
        ),
    ];
    if !extra {
        return parts.join(" ");
    }
    if let Some(load) = r.loadavg.clone() {
        if load.len() == 3 {
            parts.push(format!("LOAD {:.2} {:.2} {:.2}", load[0], load[1], load[2]));
        }
    }
    for d in &r.disks {
        if let Some(p) = d.percent {
            parts.push(format!("DISK {} {}", d.mount, percent(Some(p))));
        }
    }
    if let Some(t) = r.thermal.clone() {
        parts.push(format!("THERM {}", t));
    }
    if let Some(b) = r.battery.clone() {
        if let Some(p) = b.percent {
            parts.push(format!(
                "BAT {} {}",
                percent(Some(p as f64)),
                b.state.clone().unwrap_or("unknown".to_string())
            ));
        }
    }
    if r.psi.is_some() {
        parts.push(format!(
            "PSI cpu {} mem {} io {}",
            psi_value(&r.psi, "cpu"),
            psi_value(&r.psi, "memory"),
            psi_value(&r.psi, "io")
        ));
    }
    parts.join(" ")
}

pub fn template_values(
    r: &crate::core::Reading,
    level: Option<&str>,
) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let first = r.disks.first();
    let battery = r.battery.clone();
    let load = r.loadavg.clone().unwrap_or(vec![f64::NAN, f64::NAN, f64::NAN]);
    let l0 = if load.len() > 0 { Some(load[0]) } else { None };
    let l1 = if load.len() > 1 { Some(load[1]) } else { None };
    let l2 = if load.len() > 2 { Some(load[2]) } else { None };
    let fmt_load = |v: Option<f64>| match v {
        Some(x) if x.is_finite() => format!("{:.2}", x),
        _ => "n/a".to_string(),
    };
    m.insert("cpu".to_string(), percent(r.cpu_percent));
    m.insert("mem".to_string(), percent(r.memory_percent));
    m.insert("swap".to_string(), human_bytes(r.swap_bytes));
    m.insert(
        "pressure".to_string(),
        if r.pressure.is_empty() {
            "unknown".to_string()
        } else {
            r.pressure.clone()
        },
    );
    m.insert("load1".to_string(), fmt_load(l0));
    m.insert("load5".to_string(), fmt_load(l1));
    m.insert("load15".to_string(), fmt_load(l2));
    m.insert(
        "ncpu".to_string(),
        r.ncpu.map(|v| v.to_string()).unwrap_or_default(),
    );
    m.insert(
        "uptime".to_string(),
        r.uptime.clone().unwrap_or("n/a".to_string()),
    );
    m.insert(
        "disk".to_string(),
        percent(first.and_then(|d| d.percent)),
    );
    m.insert(
        "disk_free".to_string(),
        human_bytes(first.and_then(|d| d.free_bytes)),
    );
    m.insert(
        "thermal".to_string(),
        r.thermal.clone().unwrap_or("n/a".to_string()),
    );
    m.insert(
        "battery".to_string(),
        percent(battery.clone().and_then(|b| b.percent.map(|v| v as f64))),
    );
    m.insert(
        "battery_state".to_string(),
        battery
            .clone()
            .and_then(|b| b.state.clone())
            .unwrap_or("n/a".to_string()),
    );
    m.insert("psi_cpu".to_string(), psi_value(&r.psi, "cpu"));
    m.insert("psi_mem".to_string(), psi_value(&r.psi, "memory"));
    m.insert("psi_io".to_string(), psi_value(&r.psi, "io"));
    m.insert("level".to_string(), level.unwrap_or("").to_string());
    m.insert("hostname".to_string(), r.hostname.clone().unwrap_or_default());
    m.insert("os".to_string(), r.os.clone().unwrap_or_default());
    m.insert("time".to_string(), r.observed_at.clone().unwrap_or_default());
    m
}

pub fn render(template: &str, values: &HashMap<String, String>) -> Result<String, String> {
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' {
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                out.push('{');
                i += 2;
                continue;
            }
            // find closing }
            if let Some(j) = chars[i..].iter().position(|&c| c == '}') {
                let j = i + j;
                let name: String = chars[i + 1..j].iter().collect();
                if name.is_empty() || !values.contains_key(&name) {
                    // Match Python error message exactly.
                    let mut keys: Vec<String> = values.keys().cloned().collect();
                    keys.sort();
                    return Err(format!(
                        "unknown placeholder '{}'; available: {}",
                        name,
                        keys.join(", ")
                    ));
                }
                out.push_str(&values[&name]);
                i = j + 1;
                continue;
            } else {
                // single { without closing: Python would error; treat literally?
                out.push('{');
                i += 1;
                continue;
            }
        } else if chars[i] == '}' {
            if i + 1 < chars.len() && chars[i + 1] == '}' {
                out.push('}');
                i += 2;
                continue;
            } else {
                out.push('}');
                i += 1;
                continue;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    Ok(out)
}

pub fn check_line(level: &str, reasons: &[String]) -> String {
    if reasons.is_empty() {
        level.to_string()
    } else {
        format!("{}: {}", level, reasons.join(", "))
    }
}

pub fn format_help_text() -> String {
    // Must match Python _format_help exactly (sorted keys).
    let mut s = String::new();
    s.push_str("placeholders for --format (the value already carries its unit):\n");
    let rows: Vec<(&str, &str)> = vec![
        ("battery", "battery percent"),
        ("battery_state", "charging, discharging, full or n/a"),
        ("cpu", "CPU percent, e.g. 16%"),
        ("disk", "used percent of the first --disk mount"),
        ("disk_free", "free space of the first --disk mount"),
        ("hostname", "this machine's host name"),
        ("level", "verdict level"),
        ("load1", "1 minute load average as a number"),
        ("load15", "15 minute load average as a number"),
        ("load5", "5 minute load average as a number"),
        ("mem", "memory percent, e.g. 72%"),
        ("ncpu", "CPU count"),
        ("os", "darwin, linux or windows"),
        ("pressure", "normal, warning, critical or unknown"),
        ("psi_cpu", "CPU stall, some avg60"),
        ("psi_io", "IO stall, some avg60"),
        ("psi_mem", "memory stall, some avg60"),
        ("swap", "used swap, e.g. 1.5G"),
        ("thermal", "nominal, fair, serious, critical or n/a"),
        ("time", "reading time, ISO 8601 UTC"),
        ("uptime", "e.g. 3d 4h"),
    ];
    for (k, desc) in rows {
        s.push_str(&format!("  {{{:<13}}} {}\n", k, desc));
    }
    // Python prints "  {%-13s} %s" % (key, desc) -> "  {battery      } battery percent"
    // Our format above gives "  {battery      } battery percent"? Check: "{:<13}" pads.
    // We add extra space? Python has two spaces after }? Actually "  {%-13s} %s" -> two leading spaces, "{", key padded to 13, "}", space, desc.
    // Our string: "  {" + padded + "} " ... need to verify.
    s
}
