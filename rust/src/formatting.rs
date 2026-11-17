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
            let units: [(&str, f64); 4] = [
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

pub fn template_values(r: &crate::core::Reading, level: Option<&str>) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let first = r.disks.first();
    let battery = r.battery.clone();
    let load = r
        .loadavg
        .clone()
        .unwrap_or(vec![f64::NAN, f64::NAN, f64::NAN]);
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
    m.insert("disk".to_string(), percent(first.and_then(|d| d.percent)));
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
    m.insert(
        "hostname".to_string(),
        r.hostname.clone().unwrap_or_default(),
    );
    m.insert("os".to_string(), r.os.clone().unwrap_or_default());
    m.insert(
        "time".to_string(),
        r.observed_at.clone().unwrap_or_default(),
    );
    m
}

pub fn render(template: &str, values: &HashMap<String, String>) -> Result<String, String> {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                if chars.get(i + 1) == Some(&'{') {
                    out.push('{');
                    i += 2;
                    continue;
                }
                let (text, next) = field(&chars, i, values)?;
                out.push_str(&text);
                i = next;
            }
            '}' => {
                if chars.get(i + 1) == Some(&'}') {
                    out.push('}');
                    i += 2;
                    continue;
                }
                return Err("Single '}' encountered in format string".to_string());
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(out)
}

/// The error Python's `KeyError` becomes in `render`, so both editions print the same line.
fn unknown(name: &str, values: &HashMap<String, String>) -> String {
    let mut keys: Vec<&str> = values.keys().map(String::as_str).collect();
    keys.sort_unstable();
    format!(
        "unknown placeholder '{}'; available: {}",
        name,
        keys.join(", ")
    )
}

fn is_align(c: char) -> bool {
    matches!(c, '<' | '>' | '=' | '^')
}

/// One replacement field that starts at `start` (the `{`): its text, and the index after `}`.
///
/// Follows CPython's `str.format` for this mapping: field name, attribute or index access,
/// `!s`/`!r`/`!a` conversion, and a format spec. Values are strings, as `template_values` builds
/// them, so an attribute access always fails and an index is a character.
fn field(
    chars: &[char],
    start: usize,
    values: &HashMap<String, String>,
) -> Result<(String, usize), String> {
    let mut i = start + 1;
    let name_start = i;
    while i < chars.len() && !matches!(chars[i], '.' | '[' | '!' | ':' | '}') {
        i += 1;
    }
    let name: String = chars[name_start..i].iter().collect();
    if name.is_empty() {
        return Err(if i >= chars.len() {
            "Single '{' encountered in format string".to_string()
        } else {
            "Format string contains positional fields".to_string()
        });
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return Err("Format string contains positional fields".to_string());
    }
    let mut value = match values.get(&name) {
        Some(v) => v.clone(),
        None => return Err(unknown(&name, values)),
    };

    // Attribute access always fails on a string; index access yields one character.
    while i < chars.len() && (chars[i] == '.' || chars[i] == '[') {
        if i + 1 >= chars.len() {
            return Err("expected '}' before end of string".to_string());
        }
        let open = chars[i];
        let mut j = i + 1;
        while j < chars.len() && !matches!(chars[j], '[' | ']' | '!' | ':' | '}' | '.') {
            j += 1;
        }
        let part: String = chars[i + 1..j].iter().collect();
        if open == '.' {
            return Err(format!("'str' object has no attribute '{part}'"));
        }
        if j >= chars.len() || chars[j] != ']' {
            return Err("expected ']' before end of string".to_string());
        }
        // Python reads an all-digit index as a number, and anything else as a string key.
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return Err("string indices must be integers, not 'str'".to_string());
        }
        let index: usize = part
            .parse()
            .map_err(|_| "string index out of range".to_string())?;
        value = index_char(&value, index)?;
        i = j + 1;
    }
    if i >= chars.len() {
        return Err("expected '}' before end of string".to_string());
    }

    // `!s`, `!r` or `!a`, then `:` and the spec.
    if chars[i] == '!' {
        let conv = match chars.get(i + 1) {
            Some(c) => *c,
            None => return Err("end of string while looking for conversion specifier".to_string()),
        };
        value = match conv {
            's' => value,
            'r' | 'a' => py_repr(&value),
            other => return Err(format!("Unknown conversion specifier {}", other)),
        };
        i += 2;
        if i >= chars.len() {
            return Err("expected '}' before end of string".to_string());
        }
        if chars[i] != ':' && chars[i] != '}' {
            return Err("expected ':' after conversion specifier".to_string());
        }
    }

    if chars[i] == '}' {
        return Ok((value, i + 1));
    }

    // The spec runs to the closing brace; a nested `{name}` contributes its text.
    i += 1;
    let mut spec = String::new();
    while i < chars.len() {
        match chars[i] {
            '}' => {
                return Ok((apply_spec(&spec, &value)?, i + 1));
            }
            '{' => {
                let inner_start = i + 1;
                let mut j = inner_start;
                while j < chars.len() && !matches!(chars[j], '}' | '{' | ':') {
                    j += 1;
                }
                if j >= chars.len() || chars[j] != '}' {
                    return Err("expected '}' before end of string".to_string());
                }
                let inner: String = chars[inner_start..j].iter().collect();
                if inner.is_empty() || inner.chars().all(|c| c.is_ascii_digit()) {
                    return Err("Format string contains positional fields".to_string());
                }
                match values.get(&inner) {
                    Some(v) => spec.push_str(v),
                    None => return Err(unknown(&inner, values)),
                }
                i = j + 1;
            }
            c => {
                spec.push(c);
                i += 1;
            }
        }
    }
    Err("unmatched '{' in format spec".to_string())
}

fn index_char(value: &str, index: usize) -> Result<String, String> {
    let chars: Vec<char> = value.chars().collect();
    match chars.get(index) {
        Some(c) => Ok(c.to_string()),
        None => Err("string index out of range".to_string()),
    }
}

/// CPython's `repr` for a string, printable ASCII and common escapes.
fn py_repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Python's format spec for a string value: fill, align, width, precision and `s`.
fn apply_spec(spec: &str, value: &str) -> Result<String, String> {
    let chars: Vec<char> = spec.chars().collect();
    let invalid = || {
        format!(
            "Invalid format specifier '{}' for object of type 'str'",
            spec
        )
    };
    let mut i = 0usize;
    let mut fill = ' ';
    let mut align: Option<char> = None;
    if chars.len() >= 2 && is_align(chars[1]) {
        fill = chars[0];
        align = Some(chars[1]);
        i = 2;
    } else if !chars.is_empty() && is_align(chars[0]) {
        align = Some(chars[0]);
        i = 1;
    }
    match chars.get(i) {
        Some('+') | Some('-') => return Err("Sign not allowed in string format specifier".into()),
        Some(' ') => return Err("Space not allowed in string format specifier".into()),
        _ => {}
    }
    if chars.get(i) == Some(&'#') {
        return Err("Alternate form (#) not allowed in string format specifier".into());
    }
    if chars.get(i) == Some(&'0') {
        if align.is_none() {
            fill = '0';
            align = Some('=');
        }
        i += 1;
    }
    let mut width: Option<usize> = None;
    let width_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i > width_start {
        width = chars[width_start..i]
            .iter()
            .collect::<String>()
            .parse()
            .ok();
    }
    if matches!(chars.get(i), Some(',') | Some('_')) {
        let group = chars[i];
        i += 1;
        let with = chars.get(i).copied().unwrap_or('s');
        return Err(format!("Cannot specify '{}' with '{}'.", group, with));
    }
    let mut precision: Option<usize> = None;
    if chars.get(i) == Some(&'.') {
        i += 1;
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return Err("Format specifier missing precision".into());
        }
        precision = chars[start..i].iter().collect::<String>().parse().ok();
    }
    if i < chars.len() {
        let code = chars[i];
        i += 1;
        if i < chars.len() {
            return Err(invalid());
        }
        if code != 's' {
            return Err(format!(
                "Unknown format code '{}' for object of type 'str'",
                code
            ));
        }
    }

    let mut text: String = value.chars().collect();
    if let Some(p) = precision {
        text = text.chars().take(p).collect();
    }
    let width = width.unwrap_or(0);
    let len = text.chars().count();
    if len >= width {
        return Ok(text);
    }
    let pad: String = std::iter::repeat_n(fill, width - len).collect();
    // A string aligns left unless told otherwise; `=` means a number's own alignment.
    let align = match align {
        None | Some('=') | Some('<') => '<',
        Some(a) => a,
    };
    Ok(match align {
        '>' => format!("{pad}{text}"),
        '^' => {
            let left = (width - len) / 2;
            let right = width - len - left;
            format!(
                "{}{text}{}",
                std::iter::repeat_n(fill, left).collect::<String>(),
                std::iter::repeat_n(fill, right).collect::<String>()
            )
        }
        _ => format!("{text}{pad}"),
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("cpu".to_string(), "12%".to_string());
        m.insert("os".to_string(), "darwin".to_string());
        m.insert("ncpu".to_string(), "18".to_string());
        m.insert("level".to_string(), "ok".to_string());
        m
    }

    fn r(template: &str) -> Result<String, String> {
        render(template, &values())
    }

    // The plain placeholder path, unchanged from the first Rust edition.
    #[test]
    fn placeholders_and_escapes() {
        assert_eq!(r("CPU {cpu}").unwrap(), "CPU 12%");
        assert_eq!(r("{{}}").unwrap(), "{}");
        assert_eq!(r("{{{cpu}}}").unwrap(), "{12%}");
        assert_eq!(r("A{{B}}C").unwrap(), "A{B}C");
        assert_eq!(r("no fields").unwrap(), "no fields");
    }

    // Python's format spec for a string value: fill, align, width, precision, `s`.
    #[test]
    fn specs_match_python() {
        assert_eq!(r("{os:>8}").unwrap(), "  darwin");
        assert_eq!(r("{os:<8}").unwrap(), "darwin  ");
        assert_eq!(r("{os:^8}").unwrap(), " darwin ");
        assert_eq!(r("{os:8}").unwrap(), "darwin  ");
        assert_eq!(r("{os:*^9}").unwrap(), "*darwin**");
        assert_eq!(r("{os:.3}").unwrap(), "dar");
        assert_eq!(r("{os:>8.3}").unwrap(), "     dar");
        assert_eq!(r("{os:06}").unwrap(), "darwin");
        assert_eq!(r("{ncpu:04}").unwrap(), "1800");
        assert_eq!(r("{os:.0}").unwrap(), "");
        assert_eq!(r("{os:s}").unwrap(), "darwin");
        assert_eq!(r("{os:}").unwrap(), "darwin");
        // A nested field is spliced into the spec, as CPython does.
        assert_eq!(r("{os:{ncpu}}").unwrap(), "darwin            ");
    }

    #[test]
    fn conversions() {
        assert_eq!(r("{os!s}").unwrap(), "darwin");
        assert_eq!(r("{os!r}").unwrap(), "'darwin'");
        assert_eq!(r("{os!a}").unwrap(), "'darwin'");
        assert_eq!(r("{os!r:>8}").unwrap(), "'darwin'");
    }

    // Every message below is what the Python edition prints after `host-pressure: `.
    #[test]
    fn errors_match_python() {
        assert_eq!(
            r("{missing}").unwrap_err(),
            "unknown placeholder 'missing'; available: cpu, level, ncpu, os"
        );
        assert_eq!(
            r("{ cpu }").unwrap_err(),
            "unknown placeholder ' cpu '; available: cpu, level, ncpu, os"
        );
        assert_eq!(
            r("x } y").unwrap_err(),
            "Single '}' encountered in format string"
        );
        assert_eq!(
            r("end{").unwrap_err(),
            "Single '{' encountered in format string"
        );
        assert_eq!(r("{os").unwrap_err(), "expected '}' before end of string");
        assert_eq!(r("{os:").unwrap_err(), "unmatched '{' in format spec");
        assert_eq!(
            r("{}").unwrap_err(),
            "Format string contains positional fields"
        );
        assert_eq!(
            r("{0}").unwrap_err(),
            "Format string contains positional fields"
        );
        assert_eq!(
            r("{:>4}").unwrap_err(),
            "Format string contains positional fields"
        );
        assert_eq!(
            r("{os:{}}").unwrap_err(),
            "Format string contains positional fields"
        );
        assert_eq!(
            r("{os:+6}").unwrap_err(),
            "Sign not allowed in string format specifier"
        );
        assert_eq!(
            r("{os: 6}").unwrap_err(),
            "Space not allowed in string format specifier"
        );
        assert_eq!(
            r("{os:#}").unwrap_err(),
            "Alternate form (#) not allowed in string format specifier"
        );
        assert_eq!(r("{os:,6}").unwrap_err(), "Cannot specify ',' with '6'.");
        assert_eq!(r("{os:6,}").unwrap_err(), "Cannot specify ',' with 's'.");
        assert_eq!(
            r("{os:.}").unwrap_err(),
            "Format specifier missing precision"
        );
        assert_eq!(
            r("{os:d}").unwrap_err(),
            "Unknown format code 'd' for object of type 'str'"
        );
        assert_eq!(
            r("{os:.2.3}").unwrap_err(),
            "Invalid format specifier '.2.3' for object of type 'str'"
        );
        assert_eq!(
            r("{os:s.2}").unwrap_err(),
            "Invalid format specifier 's.2' for object of type 'str'"
        );
        assert_eq!(r("{os!x}").unwrap_err(), "Unknown conversion specifier x");
        assert_eq!(
            r("{os!ss}").unwrap_err(),
            "expected ':' after conversion specifier"
        );
    }

    // Documented divergences: the Python edition raises an uncaught traceback here, so the Rust
    // edition reports a usage error instead of reproducing a crash.
    #[test]
    fn attribute_and_index_access() {
        assert_eq!(r("{os[0]}").unwrap(), "d");
        assert_eq!(
            r("{os.x}").unwrap_err(),
            "'str' object has no attribute 'x'"
        );
        assert_eq!(
            r("{os[a]}").unwrap_err(),
            "string indices must be integers, not 'str'"
        );
        assert_eq!(r("{os[9]}").unwrap_err(), "string index out of range");
    }
}
