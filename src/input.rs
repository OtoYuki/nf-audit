//! Readers for the two artefacts every Nextflow run leaves behind:
//! `execution_trace_*.txt` (TSV) and `execution_report_*.html` (embeds a JSON blob).

use crate::model::{task_from_fields, Task};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Read a Nextflow trace TSV. With nf-core's default `trace {}` config the columns are:
/// task_id, hash, native_id, name, status, exit, submit, duration, realtime, %cpu, peak_rss,
/// peak_vmem, rchar, wchar. Any extra configured fields (cpus, memory, time, ...) are picked up too.
pub fn read_trace(path: &Path) -> Result<Vec<Task>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| anyhow!("empty trace file"))?
        .split('\t')
        .map(|s| s.trim())
        .collect();
    if !header.contains(&"name") {
        return Err(anyhow!(
            "{} is not a Nextflow trace: its first line has no `name` column (an execution report goes to --report)",
            path.display()
        ));
    }
    let mut tasks = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < header.len() {
            eprintln!(
                "warning: line {} has {} columns, expected {}; skipped",
                n + 2,
                cols.len(),
                header.len()
            );
            continue;
        }
        let fields: HashMap<String, String> = header
            .iter()
            .zip(cols.iter())
            .map(|(k, v)| (k.to_string(), v.trim().to_string()))
            .collect();
        tasks.push(task_from_fields(&fields));
    }
    Ok(tasks)
}

/// Run-level facts the report header states in prose: pipeline revision, profile, Nextflow
/// version, Fusion/Wave flags, wall-clock duration and Nextflow's own CPU-hours figure.
/// Every field is optional; older reports lack some of them.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunMeta {
    pub command: Option<String>,
    pub repository: Option<String>,
    pub revision: Option<String>,
    pub commit: Option<String>,
    pub profile: Option<String>,
    pub nextflow_version: Option<String>,
    pub wave: Option<String>,
    pub fusion: Option<String>,
    pub started: Option<String>,
    pub completed: Option<String>,
    pub duration: Option<String>,
    /// As printed by Nextflow, e.g. `316.9` or `338.4 (4.2% failed)`.
    pub cpu_hours: Option<String>,
}

impl RunMeta {
    /// One-line summary for report headers: `nf-core/rnaseq 3.15.1 · test_full_aws · Nextflow 24.04.4 · Fusion 2.3 · wall 5h 8m 46s`.
    pub fn summary_line(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(repo) = &self.repository {
            let name = repo
                .trim_end_matches('/')
                .rsplit("github.com/")
                .next()
                .unwrap_or(repo)
                .to_string();
            match &self.revision {
                Some(rev) => parts.push(format!("{name} {rev}")),
                None => parts.push(name),
            }
        }
        if let Some(p) = &self.profile {
            parts.push(p.clone());
        }
        if let Some(v) = &self.nextflow_version {
            parts.push(format!("Nextflow {v}"));
        }
        if let Some(f) = &self.fusion {
            parts.push(if f == "false" {
                "no Fusion".to_string()
            } else {
                format!("Fusion {}", f.trim_start_matches("true, version ").trim())
            });
        }
        if let Some(d) = &self.duration {
            parts.push(format!("wall {d}"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }
}

/// Extract the task records embedded in an `execution_report_*.html`, and the run facts from its header.
///
/// The report embeds `window.data = { "trace": [ {...}, ... ], "summary": {...} };` and the
/// trace records carry the *requested* resources (`cpus`, `memory`, `time`) that the default TSV
/// trace omits. Values in the JSON are strings or numbers; both are accepted.
pub fn read_report_with_meta(path: &Path) -> Result<(Vec<Task>, RunMeta)> {
    let html = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let meta = parse_run_meta(&html);
    let v = report_json(&html)?;
    let trace = match v.get("trace") {
        Some(Value::Array(t)) => t,
        Some(Value::Null) => {
            return Err(anyhow!(
                "report embeds `\"trace\": null`: the run had more tasks than Nextflow's report `maxTasks` (default 10,000). Use the trace TSV, configured with `trace.fields` to include cpus,memory,time."
            ))
        }
        _ => return Err(anyhow!("report JSON has no `trace` array")),
    };
    let mut tasks = Vec::with_capacity(trace.len());
    for rec in trace {
        let obj = match rec.as_object() {
            Some(o) => o,
            None => continue,
        };
        let fields: HashMap<String, String> = obj
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                };
                (k.clone(), s)
            })
            .collect();
        tasks.push(task_from_fields(&fields));
    }
    Ok((tasks, meta))
}

/// The report's `window.data` blob as JSON: `{"trace": [...], "summary": ...}`. Each trace
/// record keeps every field Nextflow wrote, including ones [`Task`] drops (`script`, `workdir`).
pub fn report_json(html: &str) -> Result<Value> {
    let json = js_escapes_to_json(&extract_window_data(html)?);
    serde_json::from_str(&json).context("parsing window.data JSON from report")
}

/// Pull the `<dt>label</dt><dd>value</dd>` pairs and the run-times line out of the report
/// header. Tag-stripping text extraction; the header markup has been stable since Nextflow 21.
fn parse_run_meta(html: &str) -> RunMeta {
    // Only the header precedes the first `<script>` that defines window.data; searching the
    // whole file would also be fine, but restricting it keeps this cheap on 3 MB reports.
    let head_end = find_window_data_assignment(html).unwrap_or(html.len());
    let head = &html[..head_end];
    let dd = |label: &str| -> Option<String> {
        let needle = format!(">{label}</dt>");
        let at = head.find(&needle)?;
        let rest = &head[at + needle.len()..];
        let open = rest.find("<dd")?;
        let close = rest[open..].find("</dd>")? + open;
        let inner = &rest[open..close];
        let inner = &inner[inner.find('>')? + 1..];
        let text = strip_tags(inner);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    };
    let mut m = RunMeta {
        command: dd("Nextflow command"),
        profile: dd("Workflow profile"),
        nextflow_version: dd("Nextflow version").map(|v| {
            v.trim_start_matches("version ")
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        }),
        wave: dd("Wave enabled"),
        fusion: dd("Fusion enabled"),
        cpu_hours: dd("CPU-Hours"),
        ..Default::default()
    };
    if let Some(repo) = dd("Workflow repository") {
        // `https://github.com/nf-core/rnaseq, revision 3.15.1 (commit hash 4053b2ec…)`
        let mut it = repo.splitn(2, ", revision ");
        m.repository = it.next().map(|s| s.trim().to_string());
        if let Some(rest) = it.next() {
            let mut it2 = rest.splitn(2, " (commit hash ");
            m.revision = it2.next().map(|s| s.trim().to_string());
            m.commit = it2
                .next()
                .map(|s| s.trim_end_matches(')').trim().to_string());
        }
    }
    let span = |id: &str| -> Option<String> {
        let needle = format!("id=\"{id}\">");
        let at = head.find(&needle)?;
        let rest = &head[at + needle.len()..];
        let end = rest.find("</span>")?;
        Some(rest[..end].trim().to_string()).filter(|s| !s.is_empty())
    };
    m.started = span("workflow_start");
    m.completed = span("workflow_complete");
    if let Some(at) = head.find("duration: <strong>") {
        let rest = &head[at + "duration: <strong>".len()..];
        if let Some(end) = rest.find("</strong>") {
            m.duration = Some(rest[..end].trim().to_string());
        }
    }
    m
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find the `window.data = {...};` assignment and return the balanced JSON object text.
fn extract_window_data(html: &str) -> Result<String> {
    let start = find_window_data_assignment(html).ok_or_else(|| {
        anyhow!("no `window.data = {{...}}` found in report (is this a Nextflow execution report?)")
    })?;
    let rest = &html[start..];
    let brace = rest
        .find('{')
        .ok_or_else(|| anyhow!("no object literal after window.data"))?;
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(brace) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(rest[brace..=i].to_string());
                }
            }
            _ => {}
        }
    }
    Err(anyhow!(
        "window.data never closes: the report looks truncated (an interrupted download or a run that was killed while writing it)"
    ))
}

/// Byte offset of the `window.data = {` assignment. The report's own script also contains
/// `window.data_byprocess = {}`, `window.data.summary` and `window.data.trace==null`, all of
/// which precede the real assignment, so a plain substring search lands on the wrong one.
fn find_window_data_assignment(html: &str) -> Option<usize> {
    let needle = "window.data";
    let mut from = 0;
    while let Some(i) = html[from..].find(needle) {
        let at = from + i;
        let after = html[at + needle.len()..].trim_start();
        if after.starts_with('=') && !after.starts_with("==") {
            return Some(at);
        }
        from = at + needle.len();
    }
    None
}

/// Nextflow writes the blob with a JavaScript string escaper, which emits `\'` (not valid
/// JSON) alongside `\/`, `\n`, `\"` and `\\` (all valid). Turn `\'` back into `'`, leaving
/// every other escape pair, including `\\`, untouched.
fn js_escapes_to_json(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'\'' {
                out.push(b'\'');
            } else {
                out.push(b[i]);
                out.push(b[i + 1]);
            }
            i += 2;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    // Only ASCII bytes were touched, so the result is still valid UTF-8.
    String::from_utf8(out).expect("escape rewrite preserves UTF-8")
}

/// Merge trace and report records into one task list.
///
/// Both files are written from the same `TraceRecord`s, but the report keeps exact values
/// (milliseconds, bytes) where the TSV has rounded human strings (`5m 40s`, `12.5 GB`), and it
/// carries the requested `cpus`/`memory`/`time` that the default TSV lacks. So whenever a task
/// appears in both, the report's value wins for every field it has; the trace fills what the
/// report lacks. Tasks present in only one file are kept: the trace is appended incrementally
/// and can hold tasks a report written at the end never saw, and vice versa.
///
/// A task is identified by its `hash`. The trace can log one hash more than once (in sarek's
/// megatests, an ABORTED row with no run time followed by the FAILED row); only the last row
/// counts. Matching by `name` is the fallback when one side has no hash (a trace whose
/// `trace.fields` leave it out); a hashed trace row never takes a hashed report record by name,
/// because retries of one task share its name. Each report record is used at most once.
pub fn merge(trace: Vec<Task>, report: Vec<Task>) -> Vec<Task> {
    if report.is_empty() {
        return dedup_by_hash(trace);
    }
    if trace.is_empty() {
        return report;
    }
    let trace = dedup_by_hash(trace);
    let mut by_hash: HashMap<String, usize> = HashMap::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, t) in report.iter().enumerate() {
        if !t.hash.is_empty() {
            by_hash.insert(t.hash.clone(), i);
        }
        by_name.entry(t.name.clone()).or_default().push(i);
    }
    let mut used = vec![false; report.len()];
    let mut out: Vec<Task> = Vec::with_capacity(trace.len().max(report.len()));
    for t in trace {
        // Name candidates in order, first unused wins: retries of one task share a name, so
        // their rows pair up with the report's in the order both files list them.
        let by_name_where = |ok: &dyn Fn(&Task) -> bool| {
            by_name
                .get(&t.name)
                .and_then(|v| v.iter().copied().find(|&i| !used[i] && ok(&report[i])))
        };
        let idx = if t.hash.is_empty() {
            by_name_where(&|_| true)
        } else {
            by_hash
                .get(&t.hash)
                .copied()
                .filter(|&i| !used[i])
                .or_else(|| by_name_where(&|r| r.hash.is_empty()))
        };
        match idx {
            Some(i) => {
                used[i] = true;
                out.push(prefer_report(t, &report[i]));
            }
            None => out.push(t),
        }
    }
    for (i, r) in report.into_iter().enumerate() {
        if !used[i] {
            out.push(r);
        }
    }
    out
}

/// Keep the last row per task, in order of each task's first appearance. A task is its `hash`,
/// or its `task_id` in a trace written without the hash column. Rows with neither are all kept.
fn dedup_by_hash(tasks: Vec<Task>) -> Vec<Task> {
    let mut slot: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Task> = Vec::with_capacity(tasks.len());
    for t in tasks {
        let key = if !t.hash.is_empty() {
            format!("h:{}", t.hash)
        } else if !t.task_id.is_empty() {
            format!("i:{}", t.task_id)
        } else {
            out.push(t);
            continue;
        };
        match slot.get(&key) {
            Some(&i) => out[i] = t,
            None => {
                slot.insert(key, out.len());
                out.push(t);
            }
        }
    }
    out
}

/// Combine one task seen in both files: every field the report has replaces the trace's.
fn prefer_report(t: Task, r: &Task) -> Task {
    let pick_s = |rep: &String, tr: String| if rep.is_empty() { tr } else { rep.clone() };
    Task {
        task_id: pick_s(&r.task_id, t.task_id),
        hash: pick_s(&r.hash, t.hash),
        native_id: pick_s(&r.native_id, t.native_id),
        name: pick_s(&r.name, t.name),
        process: pick_s(&r.process, t.process),
        status: pick_s(&r.status, t.status),
        exit: r.exit.or(t.exit),
        attempt: r.attempt.or(t.attempt),
        duration_s: r.duration_s.or(t.duration_s),
        realtime_s: r.realtime_s.or(t.realtime_s),
        pct_cpu: r.pct_cpu.or(t.pct_cpu),
        peak_rss: r.peak_rss.or(t.peak_rss),
        peak_vmem: r.peak_vmem.or(t.peak_vmem),
        rchar: r.rchar.or(t.rchar),
        wchar: r.wchar.or(t.wchar),
        cpus: r.cpus.or(t.cpus),
        memory: r.memory.or(t.memory),
        time_s: r.time_s.or(t.time_s),
        container: pick_s(&r.container, t.container),
        queue: pick_s(&r.queue, t.queue),
        // The report's tag field is authoritative, including when it says "untagged".
        tag: r.tag.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_balanced_object() {
        let html = r#"<script>window.data = {"trace":[{"name":"A (x)","cpus":"2","memory":"1073741824","realtime":"60000","%cpu":"150.0","peak_rss":"536870912","status":"COMPLETED","hash":"ab/cd"}],"summary":{"x":"}"}};</script>"#;
        let j = extract_window_data(html).unwrap();
        assert!(j.starts_with('{') && j.ends_with('}'));
        let v: Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["trace"][0]["cpus"], "2");
    }

    /// Shape of a real Nextflow 25.04 report: helper code referencing `window.data_byprocess`,
    /// `window.data.summary` and `window.data.trace==null` all appear before the assignment.
    #[test]
    fn skips_window_data_references_before_the_assignment() {
        let html = concat!(
            "<script>window.data_byprocess = {};\n",
            "for (let i in window.data.summary) { }\n",
            "if( window.data.trace==null ) { }\n",
            "window.data = { \"trace\":[{\"name\":\"B (y)\",\"cpus\":\"4\"}],\"summary\":[]};\n",
            "</script>"
        );
        let j = extract_window_data(html).unwrap();
        let v: Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["trace"][0]["cpus"], "4");
    }

    #[test]
    fn no_assignment_is_an_error() {
        assert!(
            extract_window_data("window.data_byprocess = {}; window.data.trace==null").is_err()
        );
    }

    /// The `script` field of every task carries shell quoting escaped JavaScript-style:
    /// `sed \'s\/x\/\/\'` arrives as `sed \\\'s\\/x\\/\\/\\\'`. `\'` is not JSON.
    #[test]
    fn rewrites_js_only_escapes() {
        let raw = r#"{"script":"sed \\\'s\/x\/\/\\\' \"q\" \n end"}"#;
        let fixed = js_escapes_to_json(raw);
        assert_eq!(fixed, r#"{"script":"sed \\'s\/x\/\/\\' \"q\" \n end"}"#);
        let v: Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(v["script"], "sed \\'s/x//\\' \"q\" \n end");
    }

    #[test]
    fn js_escape_rewrite_keeps_escaped_backslash_before_quote() {
        // `\\` then a literal `'`: the backslash pair must stay a pair and the quote must stay.
        assert_eq!(js_escapes_to_json(r"a\\'b"), r"a\\'b");
        assert_eq!(js_escapes_to_json(r"a\'b"), "a'b");
        assert_eq!(js_escapes_to_json(r"trailing\"), r"trailing\");
    }

    #[test]
    fn parses_header_facts() {
        let html = concat!(
            "<dl><dt>Run times</dt><dd><span id=\"workflow_start\">16-Sep-2024 16:33:27</span> - ",
            "<span id=\"workflow_complete\">16-Sep-2024 21:42:12</span> (<span id=\"completed_fromnow\"></span>duration: <strong>5h 8m 46s</strong>)</dd>",
            "<dt>Nextflow command</dt><dd><pre class=\"nfcommand\"><code>nextflow run x -profile test_full</code></pre></dd>",
            "<dt class=\"col-sm-3\">CPU-Hours</dt><dd class=\"col-sm-9\"><samp>316.9</samp></dd>",
            "<dt class=\"col-sm-3\">Workflow repository</dt><dd class=\"col-sm-9\"><code>https://github.com/nf-core/rnaseq</code>, revision <code>3.15.1</code> (commit hash <code>4053b2ec</code>)</dd>",
            "<dt class=\"col-sm-3\">Workflow profile</dt><dd class=\"col-sm-9\">test_full_aws</dd>",
            "<dt class=\"col-sm-3\">Fusion enabled</dt><dd class=\"col-sm-9\"><samp>true</samp>, version <samp>2.3</samp></dd>",
            "<dt class=\"col-sm-3\">Nextflow version</dt><dd class=\"col-sm-9\">version 24.04.4, build 5917 (01-08-2024 07:05 UTC)</dd></dl>",
            "<script>window.data = {\"trace\":[],\"summary\":[]};</script>"
        );
        let m = parse_run_meta(html);
        assert_eq!(m.revision.as_deref(), Some("3.15.1"));
        assert_eq!(m.commit.as_deref(), Some("4053b2ec"));
        assert_eq!(
            m.repository.as_deref(),
            Some("https://github.com/nf-core/rnaseq")
        );
        assert_eq!(m.profile.as_deref(), Some("test_full_aws"));
        assert_eq!(m.nextflow_version.as_deref(), Some("24.04.4"));
        assert_eq!(m.fusion.as_deref(), Some("true, version 2.3"));
        assert_eq!(m.cpu_hours.as_deref(), Some("316.9"));
        assert_eq!(m.duration.as_deref(), Some("5h 8m 46s"));
        assert_eq!(m.started.as_deref(), Some("16-Sep-2024 16:33:27"));
        assert_eq!(m.summary_line().as_deref(), Some("nf-core/rnaseq 3.15.1 · test_full_aws · Nextflow 24.04.4 · Fusion 2.3 · wall 5h 8m 46s"));
    }

    fn task(name: &str, hash: &str) -> Task {
        Task {
            name: name.into(),
            hash: hash.into(),
            process: Task::process_from_name(name),
            ..Default::default()
        }
    }

    #[test]
    fn merge_prefers_report_values_and_keeps_unmatched_tasks_from_both_sides() {
        let mut t1 = task("P (a)", "aa/1");
        t1.realtime_s = Some(340.0); // "5m 40s" from the TSV
        t1.status = "COMPLETED".into();
        let t_only = task("P (c)", "cc/3");
        let mut r1 = task("P (a)", "aa/1");
        r1.realtime_s = Some(339.563); // exact ms from the report
        r1.cpus = Some(6.0);
        r1.memory = Some(36.0 * 1024.0 * 1024.0 * 1024.0);
        let r_only = task("P (b)", "bb/2");
        let merged = merge(vec![t1, t_only], vec![r1, r_only]);
        assert_eq!(merged.len(), 3);
        let a = merged.iter().find(|t| t.hash == "aa/1").unwrap();
        assert_eq!(a.realtime_s, Some(339.563));
        assert_eq!(a.cpus, Some(6.0));
        assert_eq!(a.status, "COMPLETED"); // trace value kept where the report had none
        assert!(merged.iter().any(|t| t.hash == "bb/2"));
        assert!(merged.iter().any(|t| t.hash == "cc/3"));
    }

    /// A trace can log one hash twice (ABORTED then FAILED, seen in sarek `results-dev`); the
    /// report holds that task once. It must count once, with the report's record.
    #[test]
    fn merge_counts_a_repeated_trace_hash_once() {
        let mut first = task("P (a)", "aa/1");
        first.status = "ABORTED".into();
        let mut again = task("P (a)", "aa/1");
        again.status = "FAILED".into();
        again.realtime_s = Some(0.265);
        let mut r = task("P (a)", "aa/1");
        r.status = "FAILED".into();
        r.realtime_s = Some(0.265);
        let merged = merge(vec![first, again], vec![r]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "FAILED");
        // Without a matching report record, the last row for the hash is the task's final state.
        let mut a = task("P (a)", "aa/1");
        a.status = "ABORTED".into();
        let mut b = task("P (a)", "aa/1");
        b.status = "FAILED".into();
        b.realtime_s = Some(0.265);
        let only = merge(vec![a, b], vec![task("Q (z)", "zz/9")]);
        assert_eq!(only.len(), 2);
        assert_eq!(only[0].status, "FAILED");
        assert_eq!(only[0].realtime_s, Some(0.265));
    }

    /// Retries share a name but not a hash. A trace row whose hash is not in the report must not
    /// be matched to a sibling attempt by name.
    #[test]
    fn merge_does_not_match_a_retry_sibling_by_name() {
        let mut failed = task("P (s1)", "x1/1");
        failed.status = "FAILED".into();
        failed.realtime_s = Some(3600.0);
        let mut done = task("P (s1)", "x2/2");
        done.status = "COMPLETED".into();
        done.realtime_s = Some(7200.0);
        let mut r = done.clone();
        r.cpus = Some(2.0);
        let merged = merge(vec![failed, done], vec![r]);
        assert_eq!(merged.len(), 2);
        let secs: f64 = merged.iter().filter_map(|t| t.realtime_s).sum();
        assert_eq!(secs, 3.0 * 3600.0);
        assert!(merged.iter().any(|t| t.status == "FAILED"));
    }

    /// A hashless trace that logs one task twice (same `task_id`) still counts it once.
    #[test]
    fn merge_dedups_a_hashless_trace_by_task_id() {
        let mut a = task("P (a)", "");
        a.task_id = "7".into();
        a.status = "ABORTED".into();
        let mut b = a.clone();
        b.status = "FAILED".into();
        let mut r = task("P (a)", "aa/1");
        r.task_id = "7".into();
        r.status = "FAILED".into();
        let merged = merge(vec![a, b], vec![r]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "FAILED");
    }

    /// A trace written with `trace.fields` that leave out `hash` still matches its report by name.
    #[test]
    fn merge_matches_a_hashless_trace_to_a_hashed_report() {
        let t = vec![task("P (a)", ""), task("P (b)", "")];
        let mut ra = task("P (a)", "aa/1");
        ra.cpus = Some(2.0);
        let rb = task("P (b)", "bb/2");
        let merged = merge(t, vec![ra, rb]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].cpus, Some(2.0));
    }

    #[test]
    fn merge_falls_back_to_name_when_hash_is_missing() {
        let t = task("P (a)", "");
        let mut r = task("P (a)", "");
        r.cpus = Some(2.0);
        let merged = merge(vec![t], vec![r]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].cpus, Some(2.0));
    }

    #[test]
    fn a_report_is_not_read_as_a_trace() {
        let dir = std::env::temp_dir().join(format!("nf-audit-trace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("execution_report.html");
        std::fs::write(
            &p,
            "<!DOCTYPE html>\n<html>\n<script>window.data = {};</script>\n",
        )
        .unwrap();
        let err = read_trace(&p).unwrap_err().to_string();
        assert!(err.contains("not a Nextflow trace"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn null_trace_is_reported_as_max_tasks_overflow() {
        let dir = std::env::temp_dir().join(format!("nf-audit-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("report.html");
        std::fs::write(
            &p,
            "<script>window.data = {\"trace\":null,\"summary\":[]};</script>",
        )
        .unwrap();
        let err = read_report_with_meta(&p).unwrap_err().to_string();
        assert!(err.contains("maxTasks"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
