//! Pseudonymise a run's trace and report so they can be shared without naming samples, paths or
//! the people and accounts behind them.
//!
//! Every task record is rewritten field by field. Fields the analysis reads as numbers or states
//! are copied ([`KEEP`]); `process` is copied, since right-sizing is per process. A task's tag,
//! and the tag inside its `name`, becomes `tag-<hash>`; every other field (`script`, `workdir`,
//! `env`, `queue`, `container`, `native_id`, and any field this list does not know) becomes
//! `redacted-<hash>`. The hash is a salted SHA-256, so equal values get equal pseudonyms within one
//! salt, the trace and report agree, and runs redacted with the same `--salt` can be compared.
//!
//! In the report, the run name, the command line, the session, the script ID, the launch, work
//! and project directories, and a repository outside nf-core and nextflow-io are replaced in the
//! header; the report's styles and scripts are left as they are. Before anything is returned, the
//! output is scanned for every replaced value long enough to search for, and for storage URLs
//! and home-directory paths; any hit is an error.

use crate::input::{js_escapes_to_json, window_data_span};
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

/// Task fields copied as they are: numbers, states, timestamps, and the process name.
pub const KEEP: &[&str] = &[
    "task_id",
    "hash",
    "process",
    "status",
    "exit",
    "attempt",
    "submit",
    "start",
    "complete",
    "duration",
    "realtime",
    "%cpu",
    "%mem",
    "rss",
    "vmem",
    "peak_rss",
    "peak_vmem",
    "rchar",
    "wchar",
    "syscr",
    "syscw",
    "read_bytes",
    "write_bytes",
    "cpus",
    "memory",
    "disk",
    "time",
    "error_action",
    "vol_ctxt",
    "inv_ctxt",
    "cpu_model",
];

/// Report header fields whose value is replaced.
const HEADER_FIELDS: &[&str] = &[
    "Nextflow command",
    "Launch directory",
    "Work directory",
    "Project directory",
    "Workflow session",
    "Script ID",
    "Workflow container",
];

/// Header fields kept as they are. Only these are left out of the leak scan: a field this list
/// does not know is scanned, so a path in it stops the redaction.
const KEPT_FIELDS: &[&str] = &[
    "Run times",
    "CPU-Hours",
    "Nextflow version",
    "Workflow profile",
    "Wave enabled",
    "Fusion enabled",
    "Script name",
    "Workflow version",
    "Container engine",
];

/// The pipeline's manifest in the header: kept for a public repository, replaced otherwise.
const MANIFEST_FIELDS: &[&str] = &[
    "Workflow repository",
    "Workflow name",
    "Workflow home page",
    "Workflow DOI",
    "Workflow description",
    "Workflow author",
];

/// Repositories public enough to keep in the header.
const PUBLIC_REPOS: &[&str] = &[
    "https://github.com/nf-core/",
    "https://github.com/nextflow-io/",
];

/// Left in a redacted header or task record, these mean a location got through.
const LOCATION_MARKERS: &[&str] = &["s3://", "gs://", "az://", "file:/", "/home/", "/Users/"];

/// Shortest replaced value the leak scan searches for. Shorter ones (`s1`, a PID) occur by chance
/// in numbers and hashes; they are replaced by construction, field by field.
const MIN_SCAN_LEN: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Tag,
    Value,
}

#[derive(Debug, serde::Serialize)]
pub struct MapEntry {
    pub kind: Kind,
    pub pseudonym: String,
    pub original: String,
}

pub struct Redactor {
    salt: String,
    /// (kind, original) -> pseudonym
    map: BTreeMap<(Kind, String), String>,
    /// pseudonym -> original, to catch two values sharing a pseudonym
    seen: HashMap<String, String>,
}

impl Redactor {
    pub fn new(salt: &str) -> Redactor {
        Redactor {
            salt: salt.to_string(),
            map: BTreeMap::new(),
            seen: HashMap::new(),
        }
    }

    /// 128 random bits from the OS-seeded keys of two `RandomState`s, as hex.
    pub fn random_salt() -> String {
        use std::hash::{BuildHasher, Hasher};
        let mut s = String::new();
        for _ in 0..2 {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u8(0);
            s.push_str(&format!("{:016x}", h.finish()));
        }
        s
    }

    /// Number of distinct tags and other values replaced so far.
    pub fn counts(&self) -> (usize, usize) {
        let tags = self.map.keys().filter(|(k, _)| *k == Kind::Tag).count();
        (tags, self.map.len() - tags)
    }

    /// Every replacement made, for the client to keep: it reads pseudonyms back to originals.
    pub fn mapping(&self) -> Vec<MapEntry> {
        self.map
            .iter()
            .map(|((kind, original), pseudonym)| MapEntry {
                kind: *kind,
                pseudonym: pseudonym.clone(),
                original: original.clone(),
            })
            .collect()
    }

    fn pseudonym(&mut self, kind: Kind, original: &str) -> Result<String> {
        if original.is_empty() || original == "-" {
            return Ok(original.to_string());
        }
        let key = (kind, original.to_string());
        if let Some(p) = self.map.get(&key) {
            return Ok(p.clone());
        }
        let mut h = Sha256::new();
        h.update(self.salt.as_bytes());
        h.update([0]);
        h.update(original.as_bytes());
        let digest = h.finalize();
        let hex: String = digest[..5].iter().map(|b| format!("{b:02x}")).collect();
        let prefix = match kind {
            Kind::Tag => "tag",
            Kind::Value => "redacted",
        };
        let p = format!("{prefix}-{hex}");
        if let Some(other) = self.seen.get(&p) {
            if other != original {
                bail!("two values share the pseudonym {p}; run again with a different --salt");
            }
        }
        self.seen.insert(p.clone(), original.to_string());
        self.map.insert(key, p.clone());
        Ok(p)
    }

    /// `PROC (tag)` -> `PROC (tag-…)`. The tag is what follows the process name, or failing that
    /// the first ` (`, up to the closing parenthesis; a name without one is kept.
    pub fn name(&mut self, name: &str, process: Option<&str>) -> Result<String> {
        let head_len = match process.filter(|p| !p.is_empty() && *p != "-") {
            Some(p) if name.starts_with(p) && name[p.len()..].starts_with(" (") => Some(p.len()),
            _ => name.find(" ("),
        };
        match head_len {
            Some(n) if name.ends_with(')') && name.len() > n + 3 => {
                let tag = &name[n + 2..name.len() - 1];
                Ok(format!(
                    "{} ({})",
                    &name[..n],
                    self.pseudonym(Kind::Tag, tag)?
                ))
            }
            _ => Ok(name.to_string()),
        }
    }

    fn field(&mut self, key: &str, value: &str, process: Option<&str>) -> Result<String> {
        match key {
            "name" => self.name(value, process),
            "tag" => self.pseudonym(Kind::Tag, value),
            k if KEEP.contains(&k) => Ok(value.to_string()),
            _ => self.pseudonym(Kind::Value, value),
        }
    }

    /// A trace TSV, column by column. A column past the header's is replaced too.
    pub fn trace(&mut self, text: &str) -> Result<String> {
        let mut lines = text.split('\n');
        let header_line = lines.next().unwrap_or("");
        let header: Vec<&str> = header_line
            .trim_end_matches('\r')
            .split('\t')
            .map(str::trim)
            .collect();
        if !header.contains(&"name") {
            bail!("not a Nextflow trace: its first line has no `name` column");
        }
        let process_col = header.iter().position(|c| *c == "process");
        let mut out = vec![header_line.to_string()];
        for line in lines {
            let (body, cr) = match line.strip_suffix('\r') {
                Some(b) => (b, "\r"),
                None => (line, ""),
            };
            if body.trim().is_empty() {
                out.push(line.to_string());
                continue;
            }
            let cols: Vec<&str> = body.split('\t').collect();
            let process = process_col.and_then(|i| cols.get(i)).map(|s| s.trim());
            let mut new = Vec::with_capacity(cols.len());
            for (i, c) in cols.iter().enumerate() {
                let key = header.get(i).copied().unwrap_or("");
                new.push(self.field(key, c.trim(), process)?);
            }
            out.push(format!("{}{cr}", new.join("\t")));
        }
        Ok(out.join("\n"))
    }

    /// An execution report: the task records and summary labels in `window.data`, and the header.
    pub fn report(&mut self, html: &str) -> Result<String> {
        let (from, to) = window_data_span(html)?;
        let mut data: Value = serde_json::from_str(&js_escapes_to_json(&html[from..to]))
            .map_err(|e| anyhow!("parsing window.data JSON from report: {e}"))?;
        let obj = data
            .as_object_mut()
            .ok_or_else(|| anyhow!("window.data is not an object"))?;
        for (key, v) in obj.iter_mut() {
            match key.as_str() {
                "trace" => {
                    if let Value::Array(records) = v {
                        for rec in records {
                            self.record(rec)?;
                        }
                    }
                }
                // Per-process statistics; the only free text is the `*Label` task names.
                "summary" => self.labels(v)?,
                _ => self.all_strings(v)?,
            }
        }
        // `</` would end the enclosing <script> early.
        let json = serde_json::to_string(&data)?.replace("</", "<\\/");

        // The header ends at the first <script> after <body>; window.data comes later.
        let body = html.find("<body").unwrap_or(0);
        let head_end = html[body..]
            .find("<script")
            .map_or(from, |i| (body + i).min(from));
        let mut scanned = Vec::new();
        let (head, scan) = self.header(&html[..head_end], &mut scanned)?;
        self.verify(&scan, &json, &scanned)?;
        Ok(format!(
            "{head}{}{json}{}",
            &html[head_end..from],
            &html[to..]
        ))
    }

    fn record(&mut self, rec: &mut Value) -> Result<()> {
        let Some(obj) = rec.as_object_mut() else {
            return Ok(());
        };
        let process = obj
            .get("process")
            .and_then(Value::as_str)
            .map(str::to_string);
        for (k, v) in obj.iter_mut() {
            let new = match &*v {
                Value::String(s) => self.field(k, s, process.as_deref())?,
                Value::Null | Value::Bool(_) => continue,
                Value::Number(_) if KEEP.contains(&k.as_str()) => continue,
                other => self.field(k, &other.to_string(), process.as_deref())?,
            };
            *v = Value::String(new);
        }
        Ok(())
    }

    fn labels(&mut self, v: &mut Value) -> Result<()> {
        match v {
            Value::Object(m) => {
                for (k, x) in m.iter_mut() {
                    match x {
                        Value::String(s) if k.ends_with("Label") => *s = self.name(s, None)?,
                        _ => self.labels(x)?,
                    }
                }
            }
            Value::Array(a) => {
                for x in a {
                    self.labels(x)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn all_strings(&mut self, v: &mut Value) -> Result<()> {
        match v {
            Value::String(s) => *s = self.pseudonym(Kind::Value, s)?,
            Value::Object(m) => {
                for x in m.values_mut() {
                    self.all_strings(x)?;
                }
            }
            Value::Array(a) => {
                for x in a {
                    self.all_strings(x)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Replace the values of [`HEADER_FIELDS`] (and of [`MANIFEST_FIELDS`] unless the repository
    /// is public), the full error message of a failed run, and the bracketed run name everywhere
    /// in the header outside `<style>`. Returns the header and a copy for the leak scan with the
    /// fields kept on purpose blanked; originals go to `scanned`.
    fn header(&mut self, head: &str, scanned: &mut Vec<String>) -> Result<(String, String)> {
        let mut h = head.to_string();
        let public = dd_range(&h, "Workflow repository")
            .and_then(|(a, b)| first_text_node(&h[a..b]).map(|(x, y)| (a + x, a + y)))
            .is_some_and(|(a, b)| PUBLIC_REPOS.iter().any(|p| h[a..b].starts_with(p)));
        let mut replaced: Vec<&str> = HEADER_FIELDS.to_vec();
        if !public {
            replaced.extend(MANIFEST_FIELDS);
        }
        for label in &replaced {
            let Some((a, b)) = dd_range(&h, label) else {
                continue;
            };
            let new = if *label == "Workflow repository" {
                // `<code>URL</code>, revision <code>3.22.0</code> (commit hash …)`: only the URL.
                let inner = h[a..b].to_string();
                let Some((x, y)) = first_text_node(&inner) else {
                    continue;
                };
                let url = html_unescape(&inner[x..y]);
                scanned.push(url.clone());
                let p = self.pseudonym(Kind::Value, &url)?;
                format!("{}{p}{}", &inner[..x], &inner[y..])
            } else {
                // The whole value, markup included: a link keeps its URL in an attribute.
                format!("<samp>{}</samp>", self.text(&h[a..b], scanned)?)
            };
            h.replace_range(a..b, &new);
        }
        // A failed run: `<div class="alert alert-danger">…<pre>full error message</pre>`, which
        // names the task and its tag and holds the command, work directory and stderr.
        let mut from = 0;
        while let Some(i) = h[from..].find("alert-danger").map(|i| from + i) {
            let end = h[i..].find("</div>").map_or(h.len(), |j| i + j);
            let mut alert = h[i..end].to_string();
            let mut at = 0;
            while let Some(open) = alert[at..].find("<pre").map(|j| at + j) {
                let Some(gt) = alert[open..].find('>').map(|j| open + j + 1) else {
                    break;
                };
                let Some(close) = alert[gt..].find("</pre>").map(|j| gt + j) else {
                    break;
                };
                let p = self.text(&alert[gt..close], scanned)?;
                alert.replace_range(gt..close, &p);
                at = gt + p.len();
            }
            h.replace_range(i..end, &alert);
            from = i + alert.len();
        }
        // The run name: `<title>[name] …`, also in a <meta> and the page header.
        if let Some(name) = h
            .find("<title>[")
            .and_then(|i| {
                h[i + 8..]
                    .find(']')
                    .map(|j| h[i + 8..i + 8 + j].to_string())
            })
            .filter(|n| !n.is_empty())
        {
            scanned.push(name.clone());
            let p = self.pseudonym(Kind::Value, &name)?;
            h = replace_outside_style(&h, &format!("[{name}]"), &format!("[{p}]"));
        }
        let mut kept: Vec<&str> = KEPT_FIELDS.to_vec();
        if public {
            kept.extend(MANIFEST_FIELDS);
        }
        let scan = blank_kept_fields(&without_styles(&h), &kept);
        Ok((h, scan))
    }

    /// One pseudonym for the text of an HTML fragment (tags dropped, entities decoded); the
    /// original, raw and decoded, is recorded for the scan.
    fn text(&mut self, fragment: &str, scanned: &mut Vec<String>) -> Result<String> {
        let raw = html_unescape(strip_markup(fragment).trim());
        scanned.push(raw.clone());
        for line in raw.lines() {
            scanned.push(line.trim().to_string());
        }
        self.pseudonym(Kind::Value, &raw)
    }

    /// The leak scan: no replaced value of [`MIN_SCAN_LEN`] or more, raw or JSON-escaped (a
    /// multi-line script is stored escaped), and no storage URL or home path, in the redacted
    /// header or task data.
    fn verify(&self, head: &str, json: &str, scanned: &[String]) -> Result<()> {
        let mut needles: Vec<String> = Vec::new();
        for o in self
            .map
            .keys()
            .map(|(_, o)| o.as_str())
            .chain(scanned.iter().map(String::as_str))
            .filter(|o| o.trim().len() >= MIN_SCAN_LEN)
        {
            let escaped = serde_json::to_string(o)?;
            needles.push(escaped[1..escaped.len() - 1].to_string());
            needles.push(html_escape(o));
            needles.push(o.to_string());
        }
        needles.sort_unstable();
        needles.dedup();
        for (region, place) in [(head, "report header"), (json, "task data")] {
            for n in &needles {
                if let Some(i) = region.find(n.as_str()) {
                    bail!(
                        "redaction left a replaced value ({}) in the {place}, near {}; nothing was written",
                        preview(n),
                        context(region, i)
                    );
                }
            }
            for m in LOCATION_MARKERS {
                if let Some(i) = region.find(m) {
                    bail!(
                        "redaction left a location (`{m}…`) in the {place}, near {}; nothing was written",
                        context(region, i)
                    );
                }
            }
        }
        Ok(())
    }
}

/// Byte range of the first non-blank text node in an HTML fragment.
fn first_text_node(s: &str) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with('<') {
            i += s[i..].find('>')? + 1;
            continue;
        }
        let end = s[i..].find('<').map_or(s.len(), |j| i + j);
        let text = &s[i..end];
        if !text.trim().is_empty() {
            let lead = text.len() - text.trim_start().len();
            return Some((i + lead, i + lead + text.trim().len()));
        }
        i = end;
    }
    None
}

/// Byte range of the value (`<dd …>` inner) for a `<dt>label</dt>` in the header.
fn dd_range(h: &str, label: &str) -> Option<(usize, usize)> {
    let needle = format!(">{label}</dt>");
    let after = h.find(&needle)? + needle.len();
    let open = after + h[after..].find("<dd")?;
    let close = open + h[open..].find("</dd>")?;
    let gt = open + h[open..close].find('>')? + 1;
    Some((gt, close))
}

/// The header with the value of each `kept` field emptied: those are kept on purpose (pipeline
/// name, profile, versions), so a coincidental match there is not a leak.
fn blank_kept_fields(h: &str, kept: &[&str]) -> String {
    let mut out = h.to_string();
    let mut from = 0;
    while let Some(end) = out[from..].find("</dt>").map(|i| from + i) {
        let start = out[..end].rfind('>').map_or(end, |i| i + 1);
        let label = out[start..end].trim().to_string();
        from = end + 5;
        if !kept.contains(&label.as_str()) {
            continue;
        }
        if let Some((a, b)) = dd_range(&out[start.saturating_sub(1)..], &label) {
            let (a, b) = (a + start.saturating_sub(1), b + start.saturating_sub(1));
            out.replace_range(a..b, "");
            from = a;
        }
    }
    out
}

fn strip_markup(s: &str) -> String {
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
    out
}

fn without_styles(h: &str) -> String {
    let mut out = String::with_capacity(h.len());
    let mut rest = h;
    while let Some(i) = rest.find("<style") {
        out.push_str(&rest[..i]);
        rest = &rest[rest[i..].find("</style>").map_or(rest.len(), |j| i + j + 8)..];
    }
    out.push_str(rest);
    out
}

fn replace_outside_style(h: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(h.len());
    let mut rest = h;
    while let Some(i) = rest.find("<style") {
        out.push_str(&rest[..i].replace(from, to));
        let end = rest[i..].find("</style>").map_or(rest.len(), |j| i + j + 8);
        out.push_str(&rest[i..end]);
        rest = &rest[end..];
    }
    out.push_str(&rest.replace(from, to));
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

/// Up to 60 characters before a match, for an error that says where it is.
fn context(region: &str, at: usize) -> String {
    let mut from = at.saturating_sub(60);
    while !region.is_char_boundary(from) {
        from -= 1;
    }
    preview(&region[from..at])
}

fn preview(s: &str) -> String {
    let first: String = s.chars().take(40).collect();
    if first.len() < s.len() {
        format!("`{first}…`")
    } else {
        format!("`{first}`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_keep_the_process_and_replace_the_tag() {
        let mut r = Redactor::new("k");
        let a = r.name("A:B:C (S1 (lane 2))", Some("A:B:C")).unwrap();
        let b = r.name("A:B:C (S1 (lane 2))", None).unwrap();
        assert_eq!(a, b, "with and without the process column");
        assert!(a.starts_with("A:B:C (tag-") && a.ends_with(')'), "{a}");
        assert_eq!(r.name("A:B:MULTIQC", None).unwrap(), "A:B:MULTIQC");
        assert_eq!(r.field("script", "-", None).unwrap(), "-");
        assert_eq!(r.field("workdir", "", None).unwrap(), "");
        assert_eq!(r.field("%cpu", "101.4%", None).unwrap(), "101.4%");
        assert!(r
            .field("unknown_column", "x", None)
            .unwrap()
            .starts_with("redacted-"));
    }

    #[test]
    fn a_tag_and_the_same_text_elsewhere_get_distinct_prefixes() {
        let mut r = Redactor::new("k");
        let t = r.field("tag", "S1", None).unwrap();
        let v = r.field("queue", "S1", None).unwrap();
        assert_eq!(t[4..], v[9..], "same hash");
        assert!(t.starts_with("tag-") && v.starts_with("redacted-"));
        assert_eq!(r.counts(), (1, 1));
    }

    #[test]
    fn random_salts_differ() {
        assert_ne!(Redactor::random_salt(), Redactor::random_salt());
        assert_eq!(Redactor::random_salt().len(), 32);
    }

    #[test]
    fn a_header_path_the_label_list_misses_is_caught() {
        let html = concat!(
            "<html><head><title>[run_x] Nextflow Workflow Report</title></head><body>",
            "<dt>Somewhere new</dt><dd><samp>s3://acme-private/work</samp></dd>",
            "<script>window.data = {\"trace\":[],\"summary\":[]};</script></body></html>"
        );
        let e = Redactor::new("k").report(html).unwrap_err().to_string();
        assert!(e.contains("s3://"), "{e}");
    }

    #[test]
    fn header_fields_and_a_private_repository_are_replaced() {
        let html = concat!(
            "<html><head><meta content=\"run id [run_x]\"><title>[run_x] Nextflow Workflow Report</title>",
            "<style>.run_x{}</style></head><body><h2>[run_x]</h2>",
            "<dt class=\"c\">Nextflow command</dt><dd><pre><code>nextflow run acme/secret -name run_x</code></pre></dd>",
            "<dt class=\"c\">Launch directory</dt><dd class=\"c\"><samp>/srv/acme/launch</samp></dd>",
            "<dt class=\"c\">Workflow repository</dt><dd class=\"c\"><code>https://github.com/acme/secret</code>, revision <code>1.2</code> (commit hash <code>abc</code>)</dd>",
            "<dt class=\"c\">Workflow profile</dt><dd class=\"c\">test</dd>",
            "<script>window.data = {\"trace\":[{\"process\":\"P\",\"name\":\"P (S1)\",\"tag\":\"S1\",",
            "\"workdir\":\"/srv/acme/work/aa\",\"realtime\":\"5\"}],",
            "\"summary\":[{\"cpu\":{\"minLabel\":\"P (S1)\",\"max\":1.5}}]};</script></body></html>"
        );
        let out = Redactor::new("k").report(html).unwrap();
        for gone in ["run_x]", "acme/secret", "/srv/acme", "(S1)"] {
            assert!(
                !out.replace(".run_x{}", "").contains(gone),
                "{gone} in {out}"
            );
        }
        assert!(out.contains(".run_x{}"), "styles are not touched");
        assert!(out.contains("revision <code>1.2</code>"));
        assert!(out.contains("<dd class=\"c\">test</dd>"), "profile kept");
        let v = crate::input::report_json(&out).unwrap();
        assert_eq!(v["trace"][0]["realtime"], "5");
        assert_eq!(v["summary"][0]["cpu"]["max"], 1.5);
        assert_eq!(
            v["summary"][0]["cpu"]["minLabel"].as_str().unwrap(),
            v["trace"][0]["name"].as_str().unwrap()
        );
    }

    #[test]
    fn a_public_repository_is_kept() {
        let html = concat!(
            "<html><body><dt>Workflow repository</dt><dd><code>https://github.com/nf-core/rnaseq</code>, revision <code>3.22.0</code></dd>",
            "<script>window.data = {\"trace\":[],\"summary\":[]};</script></body></html>"
        );
        let out = Redactor::new("k").report(html).unwrap();
        assert!(out.contains("<code>https://github.com/nf-core/rnaseq</code>"));
    }

    #[test]
    fn trace_columns_past_the_header_and_crlf_are_handled() {
        let t = "task_id\tname\tworkdir\r\n1\tP (S1)\t/home/u/w\textra\r\n";
        let out = Redactor::new("k").trace(t).unwrap();
        let line = out.split('\n').nth(1).unwrap();
        assert!(line.ends_with('\r'), "{line:?}");
        let cols: Vec<&str> = line.trim_end().split('\t').collect();
        assert_eq!(cols[0], "1");
        assert!(cols[1].starts_with("P (tag-"));
        assert!(cols[2].starts_with("redacted-") && cols[3].starts_with("redacted-"));
    }
}
