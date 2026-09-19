//! Core task record and the parsers for Nextflow's human-formatted values.

use std::collections::HashMap;

/// One task execution, normalised from either the trace TSV or the report JSON.
///
/// Every default trace column is kept even where the current analysis does not read it, so
/// the record is complete for JSON output and per-sample attribution later.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Task {
    pub task_id: String,
    pub hash: String,
    pub native_id: String,
    /// Full task name, e.g. `NFCORE_RNASEQ:RNASEQ:ALIGN_STAR:STAR_ALIGN (SRX1603629_T1)`
    pub name: String,
    /// Process name without the tag, e.g. `NFCORE_RNASEQ:RNASEQ:ALIGN_STAR:STAR_ALIGN`
    pub process: String,
    pub status: String,
    pub exit: Option<i64>,
    pub attempt: Option<u32>,
    /// Wall time from submission to completion, in seconds.
    pub duration_s: Option<f64>,
    /// Execution time of the task itself, in seconds. This is what compute is billed on.
    pub realtime_s: Option<f64>,
    /// Average CPU utilisation in percent (100 = one core fully busy).
    pub pct_cpu: Option<f64>,
    /// Peak resident set size, bytes.
    pub peak_rss: Option<f64>,
    pub peak_vmem: Option<f64>,
    pub rchar: Option<f64>,
    pub wchar: Option<f64>,
    /// Requested CPUs (only present in report JSON, or in a trace configured with `fields`).
    pub cpus: Option<f64>,
    /// Requested memory, bytes.
    pub memory: Option<f64>,
    /// Requested time limit, seconds.
    pub time_s: Option<f64>,
    pub container: String,
    pub queue: String,
}

impl Task {
    pub fn process_from_name(name: &str) -> String {
        match name.find(" (") {
            Some(i) => name[..i].to_string(),
            None => name.to_string(),
        }
    }

    pub fn succeeded(&self) -> bool {
        matches!(self.status.as_str(), "COMPLETED" | "CACHED")
    }
}

/// Parse Nextflow's duration formatting: `1h 2m 3s`, `2.5s`, `350ms`, `1d 2h`, or a bare
/// number of milliseconds (as the report JSON uses). Returns seconds.
pub fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return None;
    }
    // Bare number: milliseconds (report JSON convention).
    if let Ok(ms) = s.parse::<f64>() {
        return Some(ms / 1000.0);
    }
    let mut total = 0.0;
    let mut num = String::new();
    let mut unit = String::new();
    let flush = |num: &mut String, unit: &mut String, total: &mut f64| -> Option<()> {
        if num.is_empty() {
            return Some(());
        }
        let v: f64 = num.parse().ok()?;
        let mult = match unit.as_str() {
            "ms" => 0.001,
            "s" | "" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            "d" => 86400.0,
            _ => return None,
        };
        *total += v * mult;
        num.clear();
        unit.clear();
        Some(())
    };
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            if !unit.is_empty() {
                flush(&mut num, &mut unit, &mut total)?;
            }
            num.push(c);
        } else if c.is_ascii_alphabetic() {
            unit.push(c);
        } else if c.is_whitespace() {
            flush(&mut num, &mut unit, &mut total)?;
        } else {
            return None;
        }
    }
    flush(&mut num, &mut unit, &mut total)?;
    Some(total)
}

/// Parse Nextflow's memory formatting: `12.5 GB`, `512 MB`, `1.2 TB`, `0`, or a bare
/// number of bytes. Nextflow's `GB` is 1024^3 (it is a MemoryUnit, binary). Returns bytes.
pub fn parse_memory(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return None;
    }
    if let Ok(b) = s.parse::<f64>() {
        return Some(b);
    }
    let (num, unit) = split_num_unit(s)?;
    let mult = match unit.to_ascii_uppercase().as_str() {
        "B" => 1.0,
        "KB" | "K" => 1024.0,
        "MB" | "M" => 1024f64.powi(2),
        "GB" | "G" => 1024f64.powi(3),
        "TB" | "T" => 1024f64.powi(4),
        "PB" | "P" => 1024f64.powi(5),
        _ => return None,
    };
    Some(num * mult)
}

/// Parse `95.3%` or `95.3`.
pub fn parse_percent(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches('%').trim();
    if s.is_empty() || s == "-" {
        return None;
    }
    s.parse().ok()
}

fn split_num_unit(s: &str) -> Option<(f64, String)> {
    let idx = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let num: f64 = s[..idx].trim().parse().ok()?;
    Some((num, s[idx..].trim().to_string()))
}

/// Build a `Task` from a string-keyed record (both the TSV row and the JSON object reduce to this).
pub fn task_from_fields(f: &HashMap<String, String>) -> Task {
    let get = |k: &str| f.get(k).map(|s| s.as_str()).unwrap_or("");
    let name = get("name").to_string();
    let process = if !get("process").is_empty() {
        get("process").to_string()
    } else {
        Task::process_from_name(&name)
    };
    Task {
        task_id: get("task_id").to_string(),
        hash: get("hash").to_string(),
        native_id: get("native_id").to_string(),
        process,
        name,
        status: get("status").to_string(),
        exit: get("exit").parse().ok(),
        attempt: get("attempt").parse().ok(),
        duration_s: parse_duration(get("duration")),
        realtime_s: parse_duration(get("realtime")),
        pct_cpu: parse_percent(get("%cpu")),
        peak_rss: parse_memory(get("peak_rss")),
        peak_vmem: parse_memory(get("peak_vmem")),
        rchar: parse_memory(get("rchar")),
        wchar: parse_memory(get("wchar")),
        cpus: get("cpus").parse().ok(),
        memory: parse_memory(get("memory")),
        time_s: parse_duration(get("time")),
        container: get("container").to_string(),
        queue: get("queue").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1h 2m 3s"), Some(3723.0));
        assert_eq!(parse_duration("2.5s"), Some(2.5));
        assert!((parse_duration("350ms").unwrap() - 0.35).abs() < 1e-9);
        assert_eq!(parse_duration("1d 2h"), Some(93600.0));
        assert_eq!(parse_duration("12345"), Some(12.345));
        assert_eq!(parse_duration("-"), None);
    }

    #[test]
    fn memory() {
        assert_eq!(parse_memory("1 GB"), Some(1073741824.0));
        assert_eq!(parse_memory("512 MB"), Some(536870912.0));
        assert_eq!(parse_memory("1024"), Some(1024.0));
        assert_eq!(
            parse_memory("12.5 GB").map(|b| (b / 1e9).round()),
            Some(13.0)
        );
    }

    #[test]
    fn percent() {
        assert_eq!(parse_percent("95.3%"), Some(95.3));
        assert_eq!(parse_percent("380.1"), Some(380.1));
    }

    #[test]
    fn process_name() {
        assert_eq!(
            Task::process_from_name("NFCORE_RNASEQ:RNASEQ:ALIGN_STAR:STAR_ALIGN (SRX1)"),
            "NFCORE_RNASEQ:RNASEQ:ALIGN_STAR:STAR_ALIGN"
        );
    }
}
