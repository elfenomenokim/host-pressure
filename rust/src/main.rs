mod core;
mod formatting;
mod verdict;
#[cfg(target_os = "windows")]
mod win;

use std::collections::HashMap;
use std::io::Write;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const EX_USAGE: i32 = 64;
const EX_TIMEOUT: i32 = 75;

fn print_usage_default() -> String {
    format!(
        "usage: host-pressure [-h] [--version] [--consumer ID] [--state-dir DIR]\n                     [--disk MOUNT] [--config PATH]\n                     [--sample SECONDS | --no-sample] [--cpu-busy CPU_BUSY]\n                     [--cpu-overloaded CPU_OVERLOADED]\n                     [--memory-busy MEMORY_BUSY]\n                     [--memory-overloaded MEMORY_OVERLOADED]\n                     [--load-busy LOAD_BUSY]\n                     [--load-overloaded LOAD_OVERLOADED]\n                     [--disk-busy DISK_BUSY]\n                     [--disk-overloaded DISK_OVERLOADED]\n                     [--swap-busy-gb SWAP_BUSY_GB]\n                     [--swap-overloaded-gb SWAP_OVERLOADED_GB] [--json]\n                     [--format TEMPLATE] [--format-help] [--all]\n                     [--watch SECONDS]\n\nPrint what this machine is doing right now, and say whether it has room.\n\noptions:\n  -h, --help            show this help message and exit\n  --version             show program's version number and exit\n  --consumer ID         name of this writer's CPU baseline (default: default)\n  --state-dir DIR       where CPU baselines are kept (default:\n                        $XDG_STATE_HOME/host-pressure)\n  --disk MOUNT          also read this mount (repeatable; default: /)\n  --config PATH         threshold file (default: $XDG_CONFIG_HOME/host-\n                        pressure/config.json)\n  --sample SECONDS      take both CPU samples in this call (e.g. 0.5); no\n                        baseline is used\n  --no-sample           never wait; print n/a when no CPU baseline exists yet\n  --cpu-busy CPU_BUSY   CPU percent that counts as busy (default 80)\n  --cpu-overloaded CPU_OVERLOADED\n                        CPU percent that counts as overloaded (default 95)\n  --memory-busy MEMORY_BUSY\n                        Memory percent that counts as busy (default 85)\n  --memory-overloaded MEMORY_OVERLOADED\n                        Memory percent that counts as overloaded (default 95)\n  --load-busy LOAD_BUSY\n                        Load average per CPU core that counts as busy (default\n                        1)\n  --load-overloaded LOAD_OVERLOADED\n                        Load average per CPU core that counts as overloaded\n                        (default 2)\n  --disk-busy DISK_BUSY\n                        Disk used percent that counts as busy (default 90)\n  --disk-overloaded DISK_OVERLOADED\n                        Disk used percent that counts as overloaded (default\n                        97)\n  --swap-busy-gb SWAP_BUSY_GB\n                        Used swap in GiB that counts as busy (default off)\n  --swap-overloaded-gb SWAP_OVERLOADED_GB\n                        Used swap in GiB that counts as overloaded (default\n                        off)\n  --json                print the full reading as JSON\n  --format TEMPLATE     print a custom line, e.g. 'CPU {{cpu}} MEM {{mem}}'; see\n                        --format-help\n  --format-help         list the --format placeholders and exit\n  --all                 add load, disk, thermal, battery and PSI fields\n  --watch SECONDS       repeat every SECONDS until interrupted\n\ncommands:\n  (none)   one line of readings          CPU 16% MEM 72% SWAP 1.5G pressure normal\n  check    ok, busy or overloaded        exit code 0, 1 or 2\n  wait     wait for a level              exit code 0, or 75 on timeout\n\nexamples:\n  host-pressure\n  host-pressure --json\n  host-pressure --all --disk /\n  host-pressure --format 'CPU {{cpu}} MEM {{mem}}'\n  host-pressure check\n  host-pressure wait --until ok --timeout 120\n"
    )
}

fn print_usage_check() -> String {
    format!(
        "usage: host-pressure check [-h] [--consumer ID] [--state-dir DIR]\n                           [--disk MOUNT] [--config PATH]\n                           [--sample SECONDS | --no-sample]\n                           [--cpu-busy CPU_BUSY]\n                           [--cpu-overloaded CPU_OVERLOADED]\n                           [--memory-busy MEMORY_BUSY]\n                           [--memory-overloaded MEMORY_OVERLOADED]\n                           [--load-busy LOAD_BUSY]\n                           [--load-overloaded LOAD_OVERLOADED]\n                           [--disk-busy DISK_BUSY]\n                           [--disk-overloaded DISK_OVERLOADED]\n                           [--swap-busy-gb SWAP_BUSY_GB]\n                           [--swap-overloaded-gb SWAP_OVERLOADED_GB] [--json]\n\nPrint ok, busy or overloaded with the readings that decided it.\n\noptions:\n  -h, --help            show this help message and exit\n  --consumer ID         name of this writer's CPU baseline (default: default)\n  --state-dir DIR       where CPU baselines are kept (default:\n                        $XDG_STATE_HOME/host-pressure)\n  --disk MOUNT          also read this mount (repeatable; default: /)\n  --config PATH         threshold file (default: $XDG_CONFIG_HOME/host-\n                        pressure/config.json)\n  --sample SECONDS      take both CPU samples in this call (e.g. 0.5); no\n                        baseline is used\n  --no-sample           never wait; print n/a when no CPU baseline exists yet\n  --cpu-busy CPU_BUSY   CPU percent that counts as busy (default 80)\n  --cpu-overloaded CPU_OVERLOADED\n                        CPU percent that counts as overloaded (default 95)\n  --memory-busy MEMORY_BUSY\n                        Memory percent that counts as busy (default 85)\n  --memory-overloaded MEMORY_OVERLOADED\n                        Memory percent that counts as overloaded (default 95)\n  --load-busy LOAD_BUSY\n                        Load average per CPU core that counts as busy (default\n                        1)\n  --load-overloaded LOAD_OVERLOADED\n                        Load average per CPU core that counts as overloaded\n                        (default 2)\n  --disk-busy DISK_BUSY\n                        Disk used percent that counts as busy (default 90)\n  --disk-overloaded DISK_OVERLOADED\n                        Disk used percent that counts as overloaded (default\n                        97)\n  --swap-busy-gb SWAP_BUSY_GB\n                        Used swap in GiB that counts as busy (default off)\n  --swap-overloaded-gb SWAP_OVERLOADED_GB\n                        Used swap in GiB that counts as overloaded (default\n                        off)\n  --json                print the level and reasons as JSON\n\nexit code 0 = ok, 1 = busy, 2 = overloaded\n"
    )
}

fn print_usage_wait() -> String {
    format!(
        "usage: host-pressure wait [-h] [--consumer ID] [--state-dir DIR]\n                          [--disk MOUNT] [--config PATH] [--cpu-busy CPU_BUSY]\n                          [--cpu-overloaded CPU_OVERLOADED]\n                          [--memory-busy MEMORY_BUSY]\n                          [--memory-overloaded MEMORY_OVERLOADED]\n                          [--load-busy LOAD_BUSY]\n                          [--load-overloaded LOAD_OVERLOADED]\n                          [--disk-busy DISK_BUSY]\n                          [--disk-overloaded DISK_OVERLOADED]\n                          [--swap-busy-gb SWAP_BUSY_GB]\n                          [--swap-overloaded-gb SWAP_OVERLOADED_GB]\n                          [--until {{ok,busy,overloaded}}] [--timeout SECONDS]\n                          [--interval SECONDS] [--sample SECONDS] [--json]\n\nPoll until the machine reaches the wanted level, then exit 0.\n\noptions:\n  -h, --help            show this help message and exit\n  --consumer ID         name of this writer's CPU baseline (default: default)\n  --state-dir DIR       where CPU baselines are kept (default:\n                        $XDG_STATE_HOME/host-pressure)\n  --disk MOUNT          also read this mount (repeatable; default: /)\n  --config PATH         threshold file (default: $XDG_CONFIG_HOME/host-\n                        pressure/config.json)\n  --cpu-busy CPU_BUSY   CPU percent that counts as busy (default 80)\n  --cpu-overloaded CPU_OVERLOADED\n                        CPU percent that counts as overloaded (default 95)\n  --memory-busy MEMORY_BUSY\n                        Memory percent that counts as busy (default 85)\n  --memory-overloaded MEMORY_OVERLOADED\n                        Memory percent that counts as overloaded (default 95)\n  --load-busy LOAD_BUSY\n                        Load average per CPU core that counts as busy (default\n                        1)\n  --load-overloaded LOAD_OVERLOADED\n                        Load average per CPU core that counts as overloaded\n                        (default 2)\n  --disk-busy DISK_BUSY\n                        Disk used percent that counts as busy (default 90)\n  --disk-overloaded DISK_OVERLOADED\n                        Disk used percent that counts as overloaded (default\n                        97)\n  --swap-busy-gb SWAP_BUSY_GB\n                        Used swap in GiB that counts as busy (default off)\n  --swap-overloaded-gb SWAP_OVERLOADED_GB\n                        Used swap in GiB that counts as overloaded (default\n                        off)\n  --until {{ok,busy,overloaded}}\n                        wanted level; a better level also passes (default: ok)\n  --timeout SECONDS     give up after this long (default: 120)\n  --interval SECONDS    pause between polls (default: 2)\n  --sample SECONDS      CPU window for each poll (default: 0.2)\n  --json                print the final verdict as JSON\n\nexit code 0 = reached, 75 = timeout, 64 = usage error\n"
    )
}

// ---------- JSON helpers (match Python json.dumps indent=2) ----------

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{08}' => o.push_str("\\b"),
            '\u{0C}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn jf(v: f64) -> String {
    if !v.is_finite() {
        return "null".to_string();
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{:.1}", v);
    }
    // Shortest round-trip like Python repr: use ryu-less fallback via format with full precision trim.
    // format with 17 significant digits then trim.
    let s = format!("{:.17}", v);
    // Trim trailing zeros but keep at least one digit after dot if needed.
    // Instead use default Display which is already shortest in Rust.
    let d = format!("{}", v);
    // Rust Display may output scientific for large/small; Python repr would too (e.g. 1e-05 vs 1e-05).
    // Prefer Display; if it lacks '.', ensure .0 for integer-valued (handled above).
    let _ = s;
    d
}

fn opt_i(v: Option<i64>) -> String {
    match v {
        Some(x) => x.to_string(),
        None => "null".to_string(),
    }
}

fn opt_f(v: Option<f64>) -> String {
    match v {
        Some(x) => jf(x),
        None => "null".to_string(),
    }
}

fn opt_s(v: &Option<String>) -> String {
    match v {
        Some(x) => format!("\"{}\"", esc(x)),
        None => "null".to_string(),
    }
}

fn battery_json(b: &Option<core::Battery>, indent: &str) -> String {
    match b {
        None => "null".to_string(),
        Some(x) => {
            let mut s = String::new();
            s.push_str("{\n");
            s.push_str(&format!("{}  \"percent\": {},\n", indent, opt_i(x.percent)));
            s.push_str(&format!(
                "{}  \"state\": {},\n",
                indent,
                x.state.as_ref().map(|v| format!("\"{}\"", esc(v))).unwrap_or("null".to_string())
            ));
            s.push_str(&format!("{}  \"secondsLeft\": {}", indent, opt_i(x.seconds_left)));
            s.push_str(&format!("\n{}}}", indent));
            s
        }
    }
}

fn psi_json(psi: &Option<HashMap<String, HashMap<String, f64>>>, indent: &str) -> String {
    match psi {
        None => "null".to_string(),
        Some(m) => {
            if m.is_empty() {
                return "{}".to_string();
            }
            let order = ["cpu", "memory", "io"];
            let inner_order = ["some_avg10", "some_avg60", "some_avg300", "full_avg10", "full_avg60", "full_avg300"];
            let mut s = String::new();
            s.push_str("{\n");
            let mut first_res = true;
            for res in order {
                if let Some(series) = m.get(res) {
                    if !first_res {
                        s.push_str(",\n");
                    }
                    first_res = false;
                    s.push_str(&format!("{}  \"{}\": {{\n", indent, esc(res)));
                    let mut first_k = true;
                    for k in inner_order {
                        if let Some(v) = series.get(k) {
                            if !first_k {
                                s.push_str(",\n");
                            }
                            first_k = false;
                            s.push_str(&format!("{}    \"{}\": {}", indent, esc(k), jf(*v)));
                        }
                    }
                    // any extra keys not in fixed order (shouldn't happen)
                    let mut extras: Vec<&String> = series.keys().filter(|k| !inner_order.contains(&k.as_str())).collect();
                    extras.sort();
                    for k in extras {
                        if !first_k {
                            s.push_str(",\n");
                        }
                        first_k = false;
                        s.push_str(&format!("{}    \"{}\": {}", indent, esc(k), jf(series[k])));
                    }
                    s.push_str(&format!("\n{}  }}", indent));
                }
            }
            s.push_str(&format!("\n{}}}", indent));
            s
        }
    }
}

fn disks_json(disks: &[core::Disk], indent: &str) -> String {
    if disks.is_empty() {
        return "[]".to_string();
    }
    let mut s = String::new();
    s.push_str("[\n");
    for (i, d) in disks.iter().enumerate() {
        s.push_str(&format!("{}  {{\n", indent));
        s.push_str(&format!("{}    \"mount\": \"{}\",\n", indent, esc(&d.mount)));
        s.push_str(&format!("{}    \"totalBytes\": {},\n", indent, opt_i(d.total_bytes)));
        s.push_str(&format!("{}    \"usedBytes\": {},\n", indent, opt_i(d.used_bytes)));
        s.push_str(&format!("{}    \"freeBytes\": {},\n", indent, opt_i(d.free_bytes)));
        s.push_str(&format!("{}    \"percent\": {}", indent, opt_f(d.percent)));
        s.push_str(&format!("\n{}  }}", indent));
        if i + 1 < disks.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str(&format!("{}]", indent));
    s
}

fn verdict_json(level: &str, reasons: &[String]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"level\": \"{}\",\n", esc(level)));
    s.push_str("  \"reasons\": [");
    if reasons.is_empty() {
        s.push_str("]");
    } else {
        s.push('\n');
        for (i, r) in reasons.iter().enumerate() {
            s.push_str(&format!("    \"{}\"", esc(r)));
            if i + 1 < reasons.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ]");
    }
    s.push_str("\n}");
    s
}

fn reading_json(r: &core::Reading, v: &verdict::Verdict) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"observedAt\": {},\n", opt_s(&r.observed_at)));
    s.push_str(&format!("  \"os\": {},\n", opt_s(&r.os)));
    s.push_str(&format!("  \"hostname\": {},\n", opt_s(&r.hostname)));
    s.push_str(&format!("  \"ncpu\": {},\n", opt_i(r.ncpu)));
    s.push_str(&format!("  \"cpuPercent\": {},\n", opt_f(r.cpu_percent)));
    s.push_str(&format!("  \"cpuSource\": {},\n", opt_s(&r.cpu_source)));
    s.push_str(&format!("  \"memoryPercent\": {},\n", opt_f(r.memory_percent)));
    s.push_str(&format!("  \"memoryTotalBytes\": {},\n", opt_i(r.memory_total_bytes)));
    s.push_str(&format!("  \"memoryUsedBytes\": {},\n", opt_i(r.memory_used_bytes)));
    s.push_str(&format!("  \"swapBytes\": {},\n", opt_i(r.swap_bytes)));
    s.push_str(&format!("  \"swapTotalBytes\": {},\n", opt_i(r.swap_total_bytes)));
    s.push_str(&format!("  \"pressure\": \"{}\",\n", esc(&r.pressure)));
    s.push_str(&format!("  \"pressureSource\": \"{}\",\n", esc(&r.pressure_source)));
    // loadavg
    match &r.loadavg {
        None => s.push_str("  \"loadavg\": null,\n"),
        Some(l) => {
            if l.is_empty() {
                s.push_str("  \"loadavg\": [],\n");
            } else {
                s.push_str("  \"loadavg\": [\n");
                for (i, x) in l.iter().enumerate() {
                    s.push_str(&format!("    {}", jf(*x)));
                    if i + 1 < l.len() {
                        s.push(',');
                    }
                    s.push('\n');
                }
                s.push_str("  ],\n");
            }
        }
    }
    s.push_str(&format!("  \"uptime\": {},\n", opt_s(&r.uptime)));
    s.push_str(&format!("  \"uptimeSeconds\": {},\n", opt_f(r.uptime_seconds)));
    match &r.thermal {
        Some(t) => s.push_str(&format!("  \"thermal\": \"{}\",\n", esc(t))),
        None => s.push_str("  \"thermal\": null,\n"),
    }
    s.push_str(&format!("  \"battery\": {},\n", battery_json(&r.battery, "  ")));
    s.push_str(&format!("  \"psi\": {},\n", psi_json(&r.psi, "  ")));
    s.push_str(&format!("  \"disks\": {},\n", disks_json(&r.disks, "  ")));
    // verdict
    s.push_str("  \"verdict\": {\n");
    s.push_str(&format!("    \"level\": \"{}\",\n", esc(&v.level)));
    s.push_str("    \"reasons\": [");
    if v.reasons.is_empty() {
        s.push_str("]");
    } else {
        s.push('\n');
        for (i, reason) in v.reasons.iter().enumerate() {
            s.push_str(&format!("      \"{}\"", esc(reason)));
            if i + 1 < v.reasons.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("    ]");
    }
    s.push_str("\n  }\n}");
    s
}

/// Fold the pretty builders to one line with Python `json.dumps` separators, so
/// watch output is one parseable object per line.
fn compact_json(pretty: &str) -> String {
    let mut out = String::with_capacity(pretty.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in pretty.chars() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            ':' | ',' => {
                out.push(c);
                out.push(' ');
            }
            c if c.is_ascii_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

// ---------- arg parsing ----------
#[derive(Debug, Default)]
struct Common {
    consumer: String,
    state_dir: Option<String>,
    disk: Vec<String>,
    config: Option<String>,
}

#[derive(Debug, Default)]
struct Sampling {
    sample: Option<f64>,
    no_sample: bool,
}

type Overrides = HashMap<String, Option<f64>>;

fn parse_float_flag(name: &str, val: Option<&str>, prog: &str, usage: &str) -> Result<f64, i32> {
    let s = match val {
        Some(v) => v,
        None => {
            eprintln!("{}: error: argument {}: expected one argument", prog, name);
            eprint!("{}", usage);
            return Err(2);
        }
    };
    match s.parse::<f64>() {
        Ok(v) => Ok(v),
        Err(_) => {
            eprintln!("{}: error: argument {}: invalid float value: '{}'", prog, name, s);
            eprint!("{}", usage);
            Err(2)
        }
    }
}

fn take_value(args: &[String], i: &mut usize, name: &str, prog: &str, usage: &str) -> Result<String, i32> {
    *i += 1;
    if *i >= args.len() {
        eprintln!("{}: error: argument {}: expected one argument", prog, name);
        eprint!("{}", usage);
        return Err(2);
    }
    Ok(args[*i].clone())
}

fn parse_common_sampling_thresholds(
    args: &[String],
    start: usize,
    prog: &str,
    usage: &str,
    allow_no_sample: bool,
) -> Result<(Common, Sampling, Overrides, usize, bool), i32> {
    let mut common = Common {
        consumer: "default".to_string(),
        state_dir: None,
        disk: vec![],
        config: None,
    };
    let mut sampling = Sampling::default();
    let mut overrides: Overrides = HashMap::new();
    let mut json = false;
    let mut i = start;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "-h" || a == "--help" {
            if prog.ends_with("check") {
                print!("{}", print_usage_check());
            } else if prog.ends_with("wait") {
                print!("{}", print_usage_wait());
            } else {
                print!("{}", print_usage_default());
            }
            return Err(-1); // signal: help printed, exit 0
        } else if a == "--json" {
            // Parsed here, not after the loop, so a later --no-sample or
            // threshold flag is still accepted in any order like argparse.
            json = true;
        } else if a == "--consumer" {
            let v = take_value(args, &mut i, "--consumer", prog, usage)?;
            common.consumer = v;
        } else if a.starts_with("--consumer=") {
            common.consumer = a["--consumer=".len()..].to_string();
        } else if a == "--state-dir" {
            let v = take_value(args, &mut i, "--state-dir", prog, usage)?;
            common.state_dir = Some(v);
        } else if a.starts_with("--state-dir=") {
            common.state_dir = Some(a["--state-dir=".len()..].to_string());
        } else if a == "--disk" {
            let v = take_value(args, &mut i, "--disk", prog, usage)?;
            common.disk.push(v);
        } else if a.starts_with("--disk=") {
            common.disk.push(a["--disk=".len()..].to_string());
        } else if a == "--config" {
            let v = take_value(args, &mut i, "--config", prog, usage)?;
            common.config = Some(v);
        } else if a.starts_with("--config=") {
            common.config = Some(a["--config=".len()..].to_string());
        } else if a == "--sample" {
            let v = take_value(args, &mut i, "--sample", prog, usage)?;
            let f = parse_float_flag("--sample", Some(&v), prog, usage)?;
            sampling.sample = Some(f);
        } else if a.starts_with("--sample=") {
            let v = &a["--sample=".len()..];
            let f = parse_float_flag("--sample", Some(v), prog, usage)?;
            sampling.sample = Some(f);
        } else if a == "--no-sample" {
            if !allow_no_sample {
                eprintln!("{}: error: unrecognized arguments: --no-sample", prog);
                eprint!("{}", usage);
                return Err(2);
            }
            sampling.no_sample = true;
        } else if a == "--cpu-busy" {
            let v = take_value(args, &mut i, "--cpu-busy", prog, usage)?;
            overrides.insert("cpu_busy".to_string(), Some(parse_float_flag("--cpu-busy", Some(&v), prog, usage)?));
        } else if a.starts_with("--cpu-busy=") {
            let v = &a["--cpu-busy=".len()..];
            overrides.insert("cpu_busy".to_string(), Some(parse_float_flag("--cpu-busy", Some(v), prog, usage)?));
        } else if a == "--cpu-overloaded" {
            let v = take_value(args, &mut i, "--cpu-overloaded", prog, usage)?;
            overrides.insert("cpu_overloaded".to_string(), Some(parse_float_flag("--cpu-overloaded", Some(&v), prog, usage)?));
        } else if a.starts_with("--cpu-overloaded=") {
            let v = &a["--cpu-overloaded=".len()..];
            overrides.insert("cpu_overloaded".to_string(), Some(parse_float_flag("--cpu-overloaded", Some(v), prog, usage)?));
        } else if a == "--memory-busy" {
            let v = take_value(args, &mut i, "--memory-busy", prog, usage)?;
            overrides.insert("memory_busy".to_string(), Some(parse_float_flag("--memory-busy", Some(&v), prog, usage)?));
        } else if a.starts_with("--memory-busy=") {
            let v = &a["--memory-busy=".len()..];
            overrides.insert("memory_busy".to_string(), Some(parse_float_flag("--memory-busy", Some(v), prog, usage)?));
        } else if a == "--memory-overloaded" {
            let v = take_value(args, &mut i, "--memory-overloaded", prog, usage)?;
            overrides.insert("memory_overloaded".to_string(), Some(parse_float_flag("--memory-overloaded", Some(&v), prog, usage)?));
        } else if a.starts_with("--memory-overloaded=") {
            let v = &a["--memory-overloaded=".len()..];
            overrides.insert("memory_overloaded".to_string(), Some(parse_float_flag("--memory-overloaded", Some(v), prog, usage)?));
        } else if a == "--load-busy" {
            let v = take_value(args, &mut i, "--load-busy", prog, usage)?;
            overrides.insert("load_busy".to_string(), Some(parse_float_flag("--load-busy", Some(&v), prog, usage)?));
        } else if a.starts_with("--load-busy=") {
            let v = &a["--load-busy=".len()..];
            overrides.insert("load_busy".to_string(), Some(parse_float_flag("--load-busy", Some(v), prog, usage)?));
        } else if a == "--load-overloaded" {
            let v = take_value(args, &mut i, "--load-overloaded", prog, usage)?;
            overrides.insert("load_overloaded".to_string(), Some(parse_float_flag("--load-overloaded", Some(&v), prog, usage)?));
        } else if a.starts_with("--load-overloaded=") {
            let v = &a["--load-overloaded=".len()..];
            overrides.insert("load_overloaded".to_string(), Some(parse_float_flag("--load-overloaded", Some(v), prog, usage)?));
        } else if a == "--disk-busy" {
            let v = take_value(args, &mut i, "--disk-busy", prog, usage)?;
            overrides.insert("disk_busy".to_string(), Some(parse_float_flag("--disk-busy", Some(&v), prog, usage)?));
        } else if a.starts_with("--disk-busy=") {
            let v = &a["--disk-busy=".len()..];
            overrides.insert("disk_busy".to_string(), Some(parse_float_flag("--disk-busy", Some(v), prog, usage)?));
        } else if a == "--disk-overloaded" {
            let v = take_value(args, &mut i, "--disk-overloaded", prog, usage)?;
            overrides.insert("disk_overloaded".to_string(), Some(parse_float_flag("--disk-overloaded", Some(&v), prog, usage)?));
        } else if a.starts_with("--disk-overloaded=") {
            let v = &a["--disk-overloaded=".len()..];
            overrides.insert("disk_overloaded".to_string(), Some(parse_float_flag("--disk-overloaded", Some(v), prog, usage)?));
        } else if a == "--swap-busy-gb" {
            let v = take_value(args, &mut i, "--swap-busy-gb", prog, usage)?;
            overrides.insert("swap_busy_gb".to_string(), Some(parse_float_flag("--swap-busy-gb", Some(&v), prog, usage)?));
        } else if a.starts_with("--swap-busy-gb=") {
            let v = &a["--swap-busy-gb=".len()..];
            overrides.insert("swap_busy_gb".to_string(), Some(parse_float_flag("--swap-busy-gb", Some(v), prog, usage)?));
        } else if a == "--swap-overloaded-gb" {
            let v = take_value(args, &mut i, "--swap-overloaded-gb", prog, usage)?;
            overrides.insert("swap_overloaded_gb".to_string(), Some(parse_float_flag("--swap-overloaded-gb", Some(&v), prog, usage)?));
        } else if a.starts_with("--swap-overloaded-gb=") {
            let v = &a["--swap-overloaded-gb=".len()..];
            overrides.insert("swap_overloaded_gb".to_string(), Some(parse_float_flag("--swap-overloaded-gb", Some(v), prog, usage)?));
        } else {
            break;
        }
        i += 1;
    }
    if sampling.sample.is_some() && sampling.no_sample {
        eprintln!("{}: error: argument --no-sample: not allowed with argument --sample", prog);
        eprint!("{}", usage);
        return Err(2);
    }
    Ok((common, sampling, overrides, i, json))
}

// ---------- reading ----------

struct ReadArgs<'a> {
    common: &'a Common,
    sampling: &'a Sampling,
}

fn do_read(args: &ReadArgs) -> core::Reading {
    if let Some(s) = args.sampling.sample {
        let mut r = core::read_host_resources(
            &args.common.consumer,
            args.common.state_dir.as_deref(),
            if args.common.disk.is_empty() { None } else { Some(args.common.disk.clone()) },
            Some(s),
        );
        // cpuSource already set by read_host_resources for sample path
        let _ = &mut r;
        return r;
    }
    let r = core::read_host_resources(
        &args.common.consumer,
        args.common.state_dir.as_deref(),
        if args.common.disk.is_empty() { None } else { Some(args.common.disk.clone()) },
        None,
    );
    if r.cpu_percent.is_none() && !args.sampling.no_sample {
        let mut warm = core::read_host_resources(
            &args.common.consumer,
            args.common.state_dir.as_deref(),
            if args.common.disk.is_empty() { None } else { Some(args.common.disk.clone()) },
            Some(0.3),
        );
        // match Python: warm["cpuSource"] = sample if present else None (already set)
        let _ = &mut warm;
        return warm;
    }
    r
}

fn rules_for(common: &Common, overrides: &Overrides) -> Result<verdict::Thresholds, String> {
    verdict::load_thresholds(common.config.as_deref(), overrides)
}

// ---------- commands ----------

fn cmd_default(args: &[String]) -> i32 {
    let prog = "host-pressure";
    let usage = print_usage_default();
    // --version anywhere in default mode? argparse handles it as action; mimic: if --version present and no subcommand, print and exit 0.
    // But careful: --version with value? It's a flag. Check exact token.
    // Also --format-help early (anywhere).
    if args.iter().any(|a| a == "--format-help") {
        print!("{}", formatting::format_help_text());
        return 0;
    }
    // parse
    let mut common = Common { consumer: "default".to_string(), state_dir: None, disk: vec![], config: None };
    let mut sampling = Sampling::default();
    let mut overrides: Overrides = HashMap::new();
    let mut json = false;
    let mut template: Option<String> = None;
    let mut all = false;
    let mut watch: Option<f64> = None;

    // manual loop to also catch top-level flags
    let mut i = 0usize;
    // first parse common/sampling/thresholds prefix? Actually flags can intermix; simplest: loop over all.
    while i < args.len() {
        let a = args[i].as_str();
        if a == "-h" || a == "--help" {
            print!("{}", usage);
            return 0;
        } else if a == "--version" {
            println!("host-pressure {}", VERSION);
            return 0;
        } else if a == "--consumer" {
            i += 1;
            if i >= args.len() {
                eprintln!("{}: error: argument --consumer: expected one argument", prog);
                eprint!("{}", usage);
                return 2;
            }
            common.consumer = args[i].clone();
        } else if a.starts_with("--consumer=") {
            common.consumer = a["--consumer=".len()..].to_string();
        } else if a == "--state-dir" {
            i += 1;
            if i >= args.len() {
                eprintln!("{}: error: argument --state-dir: expected one argument", prog);
                eprint!("{}", usage);
                return 2;
            }
            common.state_dir = Some(args[i].clone());
        } else if a.starts_with("--state-dir=") {
            common.state_dir = Some(a["--state-dir=".len()..].to_string());
        } else if a == "--disk" {
            i += 1;
            if i >= args.len() {
                eprintln!("{}: error: argument --disk: expected one argument", prog);
                eprint!("{}", usage);
                return 2;
            }
            common.disk.push(args[i].clone());
        } else if a.starts_with("--disk=") {
            common.disk.push(a["--disk=".len()..].to_string());
        } else if a == "--config" {
            i += 1;
            if i >= args.len() {
                eprintln!("{}: error: argument --config: expected one argument", prog);
                eprint!("{}", usage);
                return 2;
            }
            common.config = Some(args[i].clone());
        } else if a.starts_with("--config=") {
            common.config = Some(a["--config=".len()..].to_string());
        } else if a == "--sample" {
            i += 1;
            if i >= args.len() {
                eprintln!("{}: error: argument --sample: expected one argument", prog);
                eprint!("{}", usage);
                return 2;
            }
            match args[i].parse::<f64>() {
                Ok(v) => sampling.sample = Some(v),
                Err(_) => {
                    eprintln!("{}: error: argument --sample: invalid float value: '{}'", prog, args[i]);
                    eprint!("{}", usage);
                    return 2;
                }
            }
        } else if a.starts_with("--sample=") {
            let v = &a["--sample=".len()..];
            match v.parse::<f64>() {
                Ok(f) => sampling.sample = Some(f),
                Err(_) => {
                    eprintln!("{}: error: argument --sample: invalid float value: '{}'", prog, v);
                    eprint!("{}", usage);
                    return 2;
                }
            }
        } else if a == "--no-sample" {
            sampling.no_sample = true;
        } else if a == "--cpu-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --cpu-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("cpu_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --cpu-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--cpu-busy=") {
            let v = &a["--cpu-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("cpu_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --cpu-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--cpu-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --cpu-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("cpu_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --cpu-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--cpu-overloaded=") {
            let v = &a["--cpu-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("cpu_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --cpu-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--memory-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --memory-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("memory_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --memory-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--memory-busy=") {
            let v = &a["--memory-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("memory_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --memory-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--memory-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --memory-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("memory_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --memory-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--memory-overloaded=") {
            let v = &a["--memory-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("memory_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --memory-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--load-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --load-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("load_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --load-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--load-busy=") {
            let v = &a["--load-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("load_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --load-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--load-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --load-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("load_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --load-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--load-overloaded=") {
            let v = &a["--load-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("load_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --load-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--disk-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --disk-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("disk_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --disk-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--disk-busy=") {
            let v = &a["--disk-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("disk_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --disk-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--disk-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --disk-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("disk_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --disk-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--disk-overloaded=") {
            let v = &a["--disk-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("disk_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --disk-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--swap-busy-gb" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --swap-busy-gb: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("swap_busy_gb".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --swap-busy-gb: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--swap-busy-gb=") {
            let v = &a["--swap-busy-gb=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("swap_busy_gb".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --swap-busy-gb: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--swap-overloaded-gb" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --swap-overloaded-gb: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("swap_overloaded_gb".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --swap-overloaded-gb: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--swap-overloaded-gb=") {
            let v = &a["--swap-overloaded-gb=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("swap_overloaded_gb".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --swap-overloaded-gb: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--json" {
            json = true;
        } else if a == "--format" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --format: expected one argument", prog); eprint!("{}", usage); return 2; }
            template = Some(args[i].clone());
        } else if a.starts_with("--format=") {
            template = Some(a["--format=".len()..].to_string());
        } else if a == "--format-help" {
            print!("{}", formatting::format_help_text());
            return 0;
        } else if a == "--all" {
            all = true;
        } else if a == "--watch" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --watch: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => watch = Some(v), Err(_) => { eprintln!("{}: error: argument --watch: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--watch=") {
            let v = &a["--watch=".len()..];
            match v.parse::<f64>() { Ok(f) => watch = Some(f), Err(_) => { eprintln!("{}: error: argument --watch: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--" {
            // end of flags; rest must be empty for default mode
            if i + 1 < args.len() {
                eprintln!("{}: error: unrecognized arguments: {}", prog, args[i+1..].join(" "));
                eprint!("{}", usage);
                return 2;
            }
            break;
        } else if a.starts_with('-') {
            eprintln!("{}: error: unrecognized arguments: {}", prog, a);
            eprint!("{}", usage);
            return 2;
        } else {
            eprintln!("{}: error: unrecognized arguments: {}", prog, a);
            eprint!("{}", usage);
            return 2;
        }
        i += 1;
    }

    if sampling.sample.is_some() && sampling.no_sample {
        eprintln!("{}: error: argument --no-sample: not allowed with argument --sample", prog);
        eprint!("{}", usage);
        return 2;
    }

    let rules = match rules_for(&common, &overrides) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("host-pressure: {}", e);
            return EX_USAGE;
        }
    };

    if let Some(w) = watch {
        let interval = w.max(0.2);
        loop {
            let ra = ReadArgs { common: &common, sampling: &sampling };
            let readings = do_read(&ra);
            let v = verdict::verdict_reading(&readings, &rules);
            if let Some(t) = template.clone() {
                let vals = formatting::template_values(&readings, Some(&v.level));
                match formatting::render(&t, &vals) {
                    Ok(line) => { println!("{}", line); let _ = std::io::stdout().flush(); }
                    Err(e) => { eprintln!("host-pressure: {}", e); return EX_USAGE; }
                }
            } else if json {
                let payload = reading_json(&readings, &v);
                println!("{}", compact_json(&payload));
                let _ = std::io::stdout().flush();
            } else {
                println!("{}", formatting::one_line(&readings, all));
                let _ = std::io::stdout().flush();
            }
            std::thread::sleep(std::time::Duration::from_secs_f64(interval));
        }
    }

    let ra = ReadArgs { common: &common, sampling: &sampling };
    let readings = do_read(&ra);
    let v = verdict::verdict_reading(&readings, &rules);
    if let Some(t) = template {
        let vals = formatting::template_values(&readings, Some(&v.level));
        match formatting::render(&t, &vals) {
            Ok(line) => println!("{}", line),
            Err(e) => {
                eprintln!("host-pressure: {}", e);
                return EX_USAGE;
            }
        }
        return 0;
    }
    if json {
        println!("{}", reading_json(&readings, &v));
        return 0;
    }
    println!("{}", formatting::one_line(&readings, all));
    0
}

fn cmd_check(args: &[String]) -> i32 {
    let prog = "host-pressure check";
    let usage = print_usage_check();
    if args.iter().any(|a| a == "--format-help") {
        print!("{}", formatting::format_help_text());
        return 0;
    }
    let res = parse_common_sampling_thresholds(args, 0, prog, &usage, true);
    let (common, sampling, overrides, idx, json) = match res {
        Ok(v) => v,
        Err(code) => {
            if code == -1 {
                return 0;
            }
            return code;
        }
    };
    // The shared parser consumed --json and every common, sampling and
    // threshold flag, so only an unknown token can remain.
    if idx < args.len() {
        eprintln!("{}: error: unrecognized arguments: {}", prog, args[idx..].join(" "));
        eprint!("{}", usage);
        return 2;
    }
    let rules = match rules_for(&common, &overrides) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("host-pressure: {}", e);
            return EX_USAGE;
        }
    };
    // validate sample value? Python allows any float; negative sample would fail in sample_cpu_percent? Actually _core.sample_cpu_percent raises ValueError for negative/non-finite -> caught as EX_USAGE.
    if let Some(s) = sampling.sample {
        if !s.is_finite() || s < 0.0 {
            eprintln!("host-pressure: sample seconds must be finite and nonnegative");
            return EX_USAGE;
        }
    }
    let ra = ReadArgs { common: &common, sampling: &sampling };
    let readings = do_read(&ra);
    let v = verdict::verdict_reading(&readings, &rules);
    if json {
        println!("{}", verdict_json(&v.level, &v.reasons));
    } else {
        println!("{}", formatting::check_line(&v.level, &v.reasons));
    }
    match v.level.as_str() {
        "ok" => 0,
        "busy" => 1,
        "overloaded" => 2,
        _ => 0,
    }
}

fn cmd_wait(args: &[String]) -> i32 {
    let prog = "host-pressure wait";
    let usage = print_usage_wait();
    let mut common = Common { consumer: "default".to_string(), state_dir: None, disk: vec![], config: None };
    let mut overrides: Overrides = HashMap::new();
    let mut until = "ok".to_string();
    let mut timeout = 120.0f64;
    let mut interval = 2.0f64;
    let mut sample = 0.2f64;
    let mut json = false;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "-h" || a == "--help" {
            print!("{}", usage);
            return 0;
        } else if a == "--consumer" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --consumer: expected one argument", prog); eprint!("{}", usage); return 2; }
            common.consumer = args[i].clone();
        } else if a.starts_with("--consumer=") {
            common.consumer = a["--consumer=".len()..].to_string();
        } else if a == "--state-dir" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --state-dir: expected one argument", prog); eprint!("{}", usage); return 2; }
            common.state_dir = Some(args[i].clone());
        } else if a.starts_with("--state-dir=") {
            common.state_dir = Some(a["--state-dir=".len()..].to_string());
        } else if a == "--disk" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --disk: expected one argument", prog); eprint!("{}", usage); return 2; }
            common.disk.push(args[i].clone());
        } else if a.starts_with("--disk=") {
            common.disk.push(a["--disk=".len()..].to_string());
        } else if a == "--config" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --config: expected one argument", prog); eprint!("{}", usage); return 2; }
            common.config = Some(args[i].clone());
        } else if a.starts_with("--config=") {
            common.config = Some(a["--config=".len()..].to_string());
        } else if a == "--cpu-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --cpu-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("cpu_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --cpu-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--cpu-busy=") {
            let v = &a["--cpu-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("cpu_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --cpu-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--cpu-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --cpu-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("cpu_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --cpu-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--cpu-overloaded=") {
            let v = &a["--cpu-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("cpu_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --cpu-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--memory-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --memory-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("memory_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --memory-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--memory-busy=") {
            let v = &a["--memory-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("memory_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --memory-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--memory-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --memory-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("memory_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --memory-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--memory-overloaded=") {
            let v = &a["--memory-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("memory_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --memory-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--load-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --load-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("load_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --load-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--load-busy=") {
            let v = &a["--load-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("load_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --load-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--load-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --load-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("load_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --load-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--load-overloaded=") {
            let v = &a["--load-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("load_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --load-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--disk-busy" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --disk-busy: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("disk_busy".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --disk-busy: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--disk-busy=") {
            let v = &a["--disk-busy=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("disk_busy".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --disk-busy: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--disk-overloaded" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --disk-overloaded: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("disk_overloaded".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --disk-overloaded: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--disk-overloaded=") {
            let v = &a["--disk-overloaded=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("disk_overloaded".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --disk-overloaded: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--swap-busy-gb" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --swap-busy-gb: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("swap_busy_gb".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --swap-busy-gb: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--swap-busy-gb=") {
            let v = &a["--swap-busy-gb=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("swap_busy_gb".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --swap-busy-gb: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--swap-overloaded-gb" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --swap-overloaded-gb: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => { overrides.insert("swap_overloaded_gb".to_string(), Some(v)); } Err(_) => { eprintln!("{}: error: argument --swap-overloaded-gb: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--swap-overloaded-gb=") {
            let v = &a["--swap-overloaded-gb=".len()..];
            match v.parse::<f64>() { Ok(f) => { overrides.insert("swap_overloaded_gb".to_string(), Some(f)); } Err(_) => { eprintln!("{}: error: argument --swap-overloaded-gb: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--until" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --until: expected one argument", prog); eprint!("{}", usage); return 2; }
            until = args[i].clone();
            if until != "ok" && until != "busy" && until != "overloaded" {
                eprintln!("{}: error: argument --until: invalid choice: '{}' (choose from 'ok', 'busy', 'overloaded')", prog, until);
                eprint!("{}", usage);
                return 2;
            }
        } else if a.starts_with("--until=") {
            until = a["--until=".len()..].to_string();
            if until != "ok" && until != "busy" && until != "overloaded" {
                eprintln!("{}: error: argument --until: invalid choice: '{}' (choose from 'ok', 'busy', 'overloaded')", prog, until);
                eprint!("{}", usage);
                return 2;
            }
        } else if a == "--timeout" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --timeout: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => timeout = v, Err(_) => { eprintln!("{}: error: argument --timeout: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--timeout=") {
            let v = &a["--timeout=".len()..];
            match v.parse::<f64>() { Ok(f) => timeout = f, Err(_) => { eprintln!("{}: error: argument --timeout: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--interval" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --interval: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => interval = v, Err(_) => { eprintln!("{}: error: argument --interval: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--interval=") {
            let v = &a["--interval=".len()..];
            match v.parse::<f64>() { Ok(f) => interval = f, Err(_) => { eprintln!("{}: error: argument --interval: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--sample" {
            i += 1;
            if i >= args.len() { eprintln!("{}: error: argument --sample: expected one argument", prog); eprint!("{}", usage); return 2; }
            match args[i].parse::<f64>() { Ok(v) => sample = v, Err(_) => { eprintln!("{}: error: argument --sample: invalid float value: '{}'", prog, args[i]); eprint!("{}", usage); return 2; } }
        } else if a.starts_with("--sample=") {
            let v = &a["--sample=".len()..];
            match v.parse::<f64>() { Ok(f) => sample = f, Err(_) => { eprintln!("{}: error: argument --sample: invalid float value: '{}'", prog, v); eprint!("{}", usage); return 2; } }
        } else if a == "--json" {
            json = true;
        } else if a.starts_with('-') {
            eprintln!("{}: error: unrecognized arguments: {}", prog, a);
            eprint!("{}", usage);
            return 2;
        } else {
            eprintln!("{}: error: unrecognized arguments: {}", prog, a);
            eprint!("{}", usage);
            return 2;
        }
        i += 1;
    }

    if !(timeout >= 0.0 && interval > 0.0 && sample >= 0.0) {
        eprintln!("host-pressure wait: --timeout and --sample must be >= 0, --interval > 0");
        return EX_USAGE;
    }
    if !sample.is_finite() || !timeout.is_finite() || !interval.is_finite() {
        eprintln!("host-pressure wait: --timeout and --sample must be >= 0, --interval > 0");
        return EX_USAGE;
    }
    let wanted = match verdict::at_least(&until) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("host-pressure: {}", e);
            return EX_USAGE;
        }
    };
    let rules = match rules_for(&common, &overrides) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("host-pressure: {}", e);
            return EX_USAGE;
        }
    };
    let sampling = Sampling { sample: Some(sample), no_sample: false };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(timeout);
    loop {
        let ra = ReadArgs { common: &common, sampling: &sampling };
        let readings = do_read(&ra);
        let v = verdict::verdict_reading(&readings, &rules);
        let got = verdict::at_least(&v.level).unwrap_or(99);
        if got <= wanted {
            if json {
                println!("{}", verdict_json(&v.level, &v.reasons));
            } else {
                println!("{}", formatting::check_line(&v.level, &v.reasons));
            }
            return 0;
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now()).as_secs_f64();
        // Python checks remaining <= 0 after verdict; mimic with instant comparison.
        if std::time::Instant::now() >= deadline {
            let line = formatting::check_line(&v.level, &v.reasons);
            eprintln!("host-pressure wait: timed out after {}s at {}", format_timeout(timeout), line);
            return EX_TIMEOUT;
        }
        let sleep_for = remaining.min(interval);
        if sleep_for > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(sleep_for));
        }
        let _ = remaining;
    }
}

fn format_timeout(t: f64) -> String {
    // Match Python %g: trim.
    if t.fract() == 0.0 && t.abs() < 1e12 {
        format!("{}", t as i64)
    } else {
        let s = format!("{}", t);
        s
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = if !argv.is_empty() && argv[0] == "check" {
        cmd_check(&argv[1..].to_vec())
    } else if !argv.is_empty() && argv[0] == "wait" {
        cmd_wait(&argv[1..].to_vec())
    } else {
        // --format-help anywhere (Python checks before parsing)
        if argv.iter().any(|a| a == "--format-help") {
            print!("{}", formatting::format_help_text());
            0
        } else {
            cmd_default(&argv)
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_json_keeps_one_object_per_line() {
        let pretty = "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ],\n  \"text\": \"x  y: z\"\n}";
        assert_eq!(
            compact_json(pretty),
            "{\"a\": 1, \"b\": [2, 3], \"text\": \"x  y: z\"}"
        );
    }

    #[test]
    fn check_accepts_options_after_json() {
        let args: Vec<String> = ["--json", "--no-sample", "--cpu-busy", "10"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (_, sampling, overrides, idx, json) =
            parse_common_sampling_thresholds(&args, 0, "host-pressure check", "usage", true)
                .expect("parse");
        assert!(json);
        assert!(sampling.no_sample);
        assert_eq!(overrides.get("cpu_busy"), Some(&Some(10.0)));
        assert_eq!(idx, args.len());
    }
}
