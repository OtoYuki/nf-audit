//! `nf-audit redact`, end to end through the binary: the redacted files carry no sample tag,
//! path, script or run name, and every figure `analyze` gives is the same as on the originals.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nf-audit"))
}

fn local() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/nextflow-local")
}

/// A fresh, empty directory under the system temp dir, removed when dropped.
struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> TempDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p =
            std::env::temp_dir().join(format!("nf-audit-{name}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn redact(trace: &Path, report: &Path, out: &Path, extra: &[&str]) -> std::process::Output {
    let o = bin()
        .arg("redact")
        .arg("--trace")
        .arg(trace)
        .arg("--report")
        .arg(report)
        .arg("--out-dir")
        .arg(out)
        .args(extra)
        .output()
        .unwrap();
    o
}

fn analyze_json(trace: &Path, report: &Path) -> Value {
    let o = bin()
        .arg("analyze")
        .arg("--trace")
        .arg(trace)
        .arg("--report")
        .arg(report)
        .arg("--json")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}

/// Everything in `analyze --json` except the tag names, which redaction replaces: totals,
/// per-process rows, recommendations, and the per-tag figures with the tag itself dropped.
fn figures(v: &Value) -> Value {
    let mut v = v.clone();
    let run = v["run"].as_object_mut().unwrap();
    let mut tags: Vec<String> = run["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            let mut g = g.clone();
            g.as_object_mut().unwrap().remove("tag");
            g.to_string()
        })
        .collect();
    tags.sort();
    run.insert("tags".into(), tags.into());
    // The command line is redacted on purpose; every other header fact is kept.
    v["meta"].as_object_mut().unwrap().remove("command");
    v
}

fn redacted_pair(dir: &Path) -> (PathBuf, PathBuf) {
    (
        dir.join("execution_trace.txt"),
        dir.join("execution_report.html"),
    )
}

#[test]
fn analysis_is_unchanged_by_redaction() {
    let d = local();
    let out = TempDir::new("same");
    let o = redact(
        &d.join("execution_trace.txt"),
        &d.join("execution_report.html"),
        &out.0,
        &[],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let (t, r) = redacted_pair(&out.0);
    let before = analyze_json(
        &d.join("execution_trace.txt"),
        &d.join("execution_report.html"),
    );
    let after = analyze_json(&t, &r);
    assert_eq!(figures(&before), figures(&after));
    // The tags themselves did change.
    let tags = |v: &Value| -> Vec<String> {
        v["run"]["tags"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|g| g["tag"].as_str().map(str::to_string))
            .collect()
    };
    assert!(tags(&before).iter().any(|t| t == "s1"));
    assert!(
        tags(&after).iter().all(|t| t.starts_with("tag-")),
        "{:?}",
        tags(&after)
    );
}

#[test]
fn no_tag_path_script_or_run_name_survives() {
    let d = local();
    let out = TempDir::new("leak");
    let o = redact(
        &d.join("execution_trace.txt"),
        &d.join("execution_report.html"),
        &out.0,
        &[],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let (t, r) = redacted_pair(&out.0);
    let html_in = std::fs::read_to_string(d.join("execution_report.html")).unwrap();
    let html = std::fs::read_to_string(&r).unwrap();
    let trace = std::fs::read_to_string(&t).unwrap();

    // Values that identify the run, taken from the original report.
    let v = nf_audit::input::report_json(&html_in).unwrap();
    let rec = &v["trace"][0];
    let workdir = rec["workdir"].as_str().unwrap();
    let script = rec["script"].as_str().unwrap().trim();
    assert!(workdir.len() > 8 && script.len() > 8);
    let title = &html_in[html_in.find("<title>[").unwrap() + 8..];
    let run_name = &title[..title.find(']').unwrap()];
    for needle in [workdir, script, run_name] {
        assert!(!html.contains(needle), "report still holds {needle:?}");
        assert!(!trace.contains(needle), "trace still holds {needle:?}");
    }
    // Header paths and the command line.
    let samp = |html: &str, label: &str| -> String {
        let at = html.find(&format!(">{label}</dt>")).unwrap();
        let s = &html[at..];
        s[s.find("<samp>").unwrap() + 6..s.find("</samp>").unwrap()].to_string()
    };
    for label in ["Launch directory", "Work directory", "Project directory"] {
        let (before, after) = (samp(&html_in, label), samp(&html, label));
        assert!(after.starts_with("redacted-"), "{label}: {after:?}");
        // Short values such as `/w` occur by chance in the report's scripts; search for the rest.
        if before.len() >= 5 {
            assert!(!html.contains(&before), "{label} {before:?} survived");
        }
    }
    assert!(!html.contains("nextflow run "), "command line survived");

    // Field by field: every tag and free-text value is a pseudonym, in both files.
    let v = nf_audit::input::report_json(&html).unwrap();
    for rec in v["trace"].as_array().unwrap() {
        assert!(rec["tag"].as_str().unwrap().starts_with("tag-"), "{rec}");
        assert!(rec["name"].as_str().unwrap().ends_with(')'));
        assert!(rec["name"].as_str().unwrap().contains(" (tag-"), "{rec}");
        for k in [
            "workdir",
            "script",
            "native_id",
            "env",
            "queue",
            "container",
        ] {
            let s = rec[k].as_str().unwrap_or("-");
            assert!(
                s == "-" || s.is_empty() || s.starts_with("redacted-"),
                "{k}: {s}"
            );
        }
    }
    for line in trace.lines().skip(1) {
        let name = line.split('\t').nth(3).unwrap();
        assert!(name.contains(" (tag-"), "{name}");
    }
    // The figures nf-audit reads from the header are kept.
    let (_, meta) = nf_audit::input::read_report_with_meta(&r).unwrap();
    assert_eq!(meta.nextflow_version.as_deref(), Some("25.04.8"));
    assert!(meta.command.unwrap().starts_with("redacted-"));
}

#[test]
fn a_salt_keeps_pseudonyms_stable_and_a_map_reverses_them() {
    let d = local();
    let (a, b, c) = (
        TempDir::new("salt-a"),
        TempDir::new("salt-b"),
        TempDir::new("salt-c"),
    );
    let map = a.0.join("map.json");
    let args = ["--salt", "k3y"];
    let mut with_map = args.to_vec();
    let map_s = map.to_str().unwrap().to_string();
    with_map.extend(["--map", map_s.as_str()]);
    for (dir, extra) in [(&a, with_map.as_slice()), (&b, &args[..]), (&c, &[][..])] {
        let o = redact(
            &d.join("execution_trace.txt"),
            &d.join("execution_report.html"),
            &dir.0,
            extra,
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let read = |dir: &TempDir| std::fs::read_to_string(dir.0.join("execution_trace.txt")).unwrap();
    assert_eq!(read(&a), read(&b), "same salt, same pseudonyms");
    assert_ne!(read(&a), read(&c), "no salt given: a random one");

    // Trace and report agree on each task's pseudonym.
    let html = std::fs::read_to_string(a.0.join("execution_report.html")).unwrap();
    let v = nf_audit::input::report_json(&html).unwrap();
    let trace = read(&a);
    for rec in v["trace"].as_array().unwrap() {
        let name = rec["name"].as_str().unwrap();
        let hash = rec["hash"].as_str().unwrap();
        if let Some(line) = trace.lines().find(|l| l.split('\t').nth(1) == Some(hash)) {
            assert_eq!(line.split('\t').nth(3), Some(name));
        }
    }

    // The map lets the client read pseudonyms back.
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&map).unwrap()).unwrap();
    let s1 = m
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "tag" && e["original"] == "s1")
        .unwrap();
    assert!(trace.contains(&format!("({})", s1["pseudonym"].as_str().unwrap())));
}

#[test]
fn refuses_to_overwrite_its_inputs() {
    let tmp = TempDir::new("inplace");
    let d = local();
    for f in ["execution_trace.txt", "execution_report.html"] {
        std::fs::copy(d.join(f), tmp.0.join(f)).unwrap();
    }
    let o = redact(
        &tmp.0.join("execution_trace.txt"),
        &tmp.0.join("execution_report.html"),
        &tmp.0,
        &[],
    );
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("would overwrite"));
    assert_eq!(
        std::fs::read(tmp.0.join("execution_trace.txt")).unwrap(),
        std::fs::read(d.join("execution_trace.txt")).unwrap()
    );
}

/// The nf-core megatest 3.22.0 `star_rsem` run: 301 tasks, sample tags such as `K562_REP1`,
/// S3 work paths and a params-file URL in the command line.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn megatest_run_redacts_and_analyses_the_same() {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "data/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info",
    );
    let (t0, r0) = (
        d.join("execution_trace_2025-11-27_10-50-37.txt"),
        d.join("execution_report_2025-11-27_10-50-37.html"),
    );
    let out = TempDir::new("megatest");
    let o = redact(&t0, &r0, &out.0, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let t = out.0.join("execution_trace_2025-11-27_10-50-37.txt");
    let r = out.0.join("execution_report_2025-11-27_10-50-37.html");
    assert_eq!(
        figures(&analyze_json(&t0, &r0)),
        figures(&analyze_json(&t, &r))
    );
    let html = std::fs::read_to_string(&r).unwrap();
    let trace = std::fs::read_to_string(&t).unwrap();
    for needle in [
        "K562",
        "GM12878",
        "MCF7",
        "H1_REP",
        "SRX1603",
        "nf-core-awsmegatests",
        "pensive_nightingale",
        "LXF5OtRCD1FBycno59fNhg",
        "TowerForge",
        "9443c377-895d-4868-8c6e-4c5eb3c5db86",
    ] {
        assert!(!html.contains(needle), "report still holds {needle}");
        assert!(!trace.contains(needle), "trace still holds {needle}");
    }
    // nf-core's public repository and the revision stay readable.
    let (_, meta) = nf_audit::input::read_report_with_meta(&r).unwrap();
    assert_eq!(meta.revision.as_deref(), Some("3.22.0"));
    assert_eq!(
        meta.repository.as_deref(),
        Some("https://github.com/nf-core/rnaseq")
    );
}
