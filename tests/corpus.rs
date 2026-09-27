//! Every figure the README and `examples/README.md` quote about the nf-core megatest corpus,
//! recomputed from the raw reports in `data/`.
//!
//! The reports are not in the repository (109 files, about 367 MB). Fetch them, then run:
//!
//! ```text
//! scripts/pull-megatests.sh --corpus
//! cargo test --release --test corpus -- --ignored
//! ```
//!
//! Each test names the document and the sentence it checks. A test fails when a document
//! states a number the data does not give, so a doc edit that changes a figure must change
//! the matching assertion too.

use nf_audit::analysis::{analyse, recommend, Rates, RunStats};
use nf_audit::input::{read_report_with_meta, report_json, RunMeta};
use nf_audit::model::Task;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
const RSEM: &str = "RSEM_CALCULATEEXPRESSION";

struct Run {
    path: PathBuf,
    html: String,
    tasks: Vec<Task>,
    meta: RunMeta,
    stats: RunStats,
}

impl Run {
    fn load(path: PathBuf) -> Run {
        assert!(
            path.exists(),
            "{} is missing: fetch the corpus with scripts/pull-megatests.sh --corpus",
            path.display()
        );
        let html = std::fs::read_to_string(&path).unwrap();
        let (tasks, meta) = read_report_with_meta(&path).unwrap();
        let stats = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        Run {
            path,
            html,
            tasks,
            meta,
            stats,
        }
    }
    fn rev(&self) -> &str {
        self.meta.revision.as_deref().unwrap_or("?")
    }
    fn process(&self, suffix: &str) -> &nf_audit::analysis::ProcessStats {
        self.stats
            .processes
            .iter()
            .find(|p| p.process.ends_with(&format!(":{suffix}")))
            .unwrap_or_else(|| panic!("{suffix} not in {}", self.path.display()))
    }
    fn share(&self, suffix: &str) -> f64 {
        self.process(suffix).cost / self.stats.total_cost * 100.0
    }
    fn unused_pct(&self) -> f64 {
        let r = Rates::SEQERA_COMPUTE;
        let metered =
            self.stats.cpu_h_req_metered * r.cpu_hour + self.stats.gib_h_req_metered * r.gib_hour;
        self.stats.total_waste / metered * 100.0
    }
    fn tasks_of(&self, suffix: &str) -> impl Iterator<Item = &Task> {
        let suffix = format!(":{suffix}");
        self.tasks
            .iter()
            .filter(move |t| t.process.ends_with(&suffix))
    }
    /// Nextflow's own `CPU-Hours` header figure; it prints thousands as `1'177.0`.
    fn header_cpu_hours(&self) -> f64 {
        let s = self.meta.cpu_hours.as_deref().expect("CPU-Hours in header");
        s.split_whitespace()
            .next()
            .unwrap()
            .replace(['\'', ','], "")
            .parse()
            .unwrap()
    }
    /// Last line of the failed task's stdout, as the report's error section quotes it under
    /// `Command output:` (Fusion appends its own `Fusion Info:` block after it).
    fn last_output_line(&self) -> Option<&str> {
        let at = self.html.rfind("Command output:")? + "Command output:".len();
        self.html[at..]
            .split("\n\n")
            .next()?
            .lines()
            .map(str::trim)
            .take_while(|l| *l != "Fusion Info:")
            .filter(|l| !l.is_empty())
            .last()
    }
    /// `N` when the failed task's stdout ends on RSEM's `Parsed N entries`.
    fn last_parsed(&self) -> Option<u64> {
        self.last_output_line()?
            .strip_prefix("Parsed ")?
            .strip_suffix(" entries")?
            .parse()
            .ok()
    }
    fn timed_out(&self) -> bool {
        self.html.contains("Job attempt duration exceeded timeout")
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Paths in `examples/corpus.sha256` that start with `prefix`, in manifest (sorted) order. The
/// manifest, not a directory listing, defines the corpus, so extra files in `data/` change nothing.
fn manifest(prefix: &str) -> Vec<PathBuf> {
    let text = std::fs::read_to_string(root().join("examples/corpus.sha256")).unwrap();
    text.lines()
        .filter_map(|l| l.split_once("  ").map(|(_, p)| p))
        .filter(|p| p.starts_with(prefix))
        .map(|p| root().join(p))
        .collect()
}

fn load_list(name: &str) -> Vec<Run> {
    let text = std::fs::read_to_string(root().join("examples").join(name)).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| Run::load(root().join(l)))
        .collect()
}

fn salmon() -> &'static [Run] {
    static R: OnceLock<Vec<Run>> = OnceLock::new();
    R.get_or_init(|| load_list("rnaseq-star_salmon-releases.files"))
}

fn rsem() -> &'static [Run] {
    static R: OnceLock<Vec<Run>> = OnceLock::new();
    R.get_or_init(|| load_list("rnaseq-star_rsem-releases.files"))
}

fn by_rev<'a>(runs: &'a [Run], rev: &str) -> &'a Run {
    let hits: Vec<&Run> = runs.iter().filter(|r| r.rev() == rev).collect();
    assert_eq!(hits.len(), 1, "expected one run of {rev}");
    hits[0]
}

/// Index of a revision in list order (the lists are in release order; 3.13.x reads `master`).
fn pos(runs: &[Run], rev: &str) -> usize {
    runs.iter().position(|r| r.rev() == rev).unwrap()
}

fn round(x: f64, dp: i32) -> f64 {
    let m = 10f64.powi(dp);
    (x * m).round() / m
}

/// Inclusive quantile (linear interpolation over n-1 intervals; Excel `QUARTILE.INC`).
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * q;
    let (lo, hi) = (h.floor() as usize, h.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (h - lo as f64)
}

fn release_ge(rev: &str, min: &[u32]) -> bool {
    let v: Vec<u32> = rev.split('.').filter_map(|x| x.parse().ok()).collect();
    !v.is_empty() && v.as_slice() >= min
}

/// Runs without data: every report a `*.files` list names is pinned in the manifest.
#[test]
fn manifest_covers_every_listed_report() {
    let pinned: BTreeSet<PathBuf> = manifest("data/").into_iter().collect();
    for list in [
        "rnaseq-star_salmon-releases.files",
        "rnaseq-star_rsem-releases.files",
    ] {
        let text = std::fs::read_to_string(root().join("examples").join(list)).unwrap();
        for l in text.lines().filter(|l| !l.trim().is_empty()) {
            assert!(
                pinned.contains(&root().join(l)),
                "{l} not in examples/corpus.sha256"
            );
        }
    }
    assert_eq!(pinned.len(), 109);
}

// ---------------------------------------------------------------------------------------------
// README.md

/// README "Status": 61 runs, 30 `star_salmon` 3.1–3.26.0 and 31 `star_rsem` 3.1–3.27.0,
/// Nextflow 21.04 to 26.04, requested CPU-hours within 0.05 of the report header on all 61.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn corpus_shape_and_header_cpu_hours() {
    let (s, r) = (salmon(), rsem());
    assert_eq!((s.len(), r.len()), (30, 31));
    assert_eq!((s[0].rev(), s[29].rev()), ("3.1", "3.26.0"));
    assert_eq!((r[0].rev(), r[30].rev()), ("3.1", "3.27.0"));
    for run in s.iter().chain(r) {
        let d = (run.stats.cpu_h_requested - run.header_cpu_hours()).abs();
        assert!(d < 0.05, "{}: {d}", run.path.display());
    }
    let nf: BTreeSet<String> = s
        .iter()
        .chain(r)
        .map(|x| x.meta.nextflow_version.clone().unwrap()[..5].to_string())
        .collect();
    assert_eq!(nf.first().unwrap(), "21.04");
    assert_eq!(nf.last().unwrap(), "26.04");
    // "Anchor to reconcile against": 5h 8m 46s and 316.9 CPU-hours, reproduced exactly.
    let a = by_rev(s, "3.15.1");
    assert_eq!(a.meta.duration.as_deref(), Some("5h 8m 46s"));
    assert_eq!(format!("{:.1}", a.stats.cpu_h_requested), "316.9");
    assert_eq!(a.meta.cpu_hours.as_deref(), Some("316.9"));
}

/// README "Cost model": 3.15.1 is $79.23 on `seqera-compute` and $6.21 on `aws-m5-spot`, about
/// 13× apart; unused 66% on `seqera-compute`, 64% on both m5 presets.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn presets_on_the_3_15_1_anchor() {
    let a = by_rev(salmon(), "3.15.1");
    let at = |r: &Rates| {
        let st = analyse(&a.tasks, r);
        let metered = st.cpu_h_req_metered * r.cpu_hour + st.gib_h_req_metered * r.gib_hour;
        (st.total_cost, st.total_waste / metered * 100.0)
    };
    let (seq, seq_u) = at(&Rates::SEQERA_COMPUTE);
    let (od, od_u) = at(&Rates::AWS_M5_ONDEMAND);
    let (spot, spot_u) = at(&Rates::AWS_M5_SPOT);
    assert_eq!(format!("{seq:.2} {spot:.2}"), "79.23 6.21");
    assert_eq!((seq / spot).round(), 13.0);
    assert_eq!(
        (seq_u.round(), od_u.round(), spot_u.round()),
        (66.0, 64.0, 64.0)
    );
    assert!(
        seq > 34.90 && spot < 34.90,
        "presets bracket Seqera's $34.90"
    );
    let _ = od;
}

/// README "Cost by task tag": on 3.22.0 `star_rsem` the eight samples carry $31.19 to $42.45
/// each of a $294.25 run.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn per_sample_cost_3_22_0() {
    let run = by_rev(rsem(), "3.22.0");
    assert_eq!(format!("{:.2}", run.stats.total_cost), "294.25");
    let samples: Vec<f64> = run
        .stats
        .tags
        .iter()
        .filter(|g| g.tag.as_deref().is_some_and(|t| t.contains("_REP")))
        .map(|g| g.cost)
        .collect();
    assert_eq!(samples.len(), 8);
    let (lo, hi) = samples
        .iter()
        .fold((f64::MAX, 0f64), |(l, h), &c| (l.min(c), h.max(c)));
    assert_eq!(format!("{lo:.2} {hi:.2}"), "31.19 42.45");
}

// ---------------------------------------------------------------------------------------------
// RSEM_CALCULATEEXPRESSION, 3.22.0 onward (README, examples/README point 3, rsem-threads/README)

fn rsem_completed_since_3_22() -> Vec<(&'static Run, &'static Task)> {
    rsem()
        .iter()
        .filter(|r| release_ge(r.rev(), &[3, 22, 0]))
        .flat_map(|r| {
            r.tasks_of(RSEM)
                .filter(|t| t.status == "COMPLETED")
                .map(move |t| (r, t))
        })
        .collect()
}

/// "RSEM tasks request 12 CPUs / 72 GiB / 16 h. Across the 45 completed tasks from 3.22.0
/// onward they average a median of 1.04 cores (mean 1.20, range 0.25–5.1) and peak at or below
/// 9.45 GiB." rsem-threads/README adds quartiles 94–131% by the inclusive method.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn rsem_usage_since_3_22() {
    for run in rsem().iter().filter(|r| release_ge(r.rev(), &[3, 22, 0])) {
        for t in run.tasks_of(RSEM).filter(|t| t.attempt == Some(1)) {
            assert_eq!(
                (t.cpus, t.memory.map(|m| m / GIB), t.time_s),
                (Some(12.0), Some(72.0), Some(16.0 * 3600.0))
            );
        }
    }
    let done = rsem_completed_since_3_22();
    assert_eq!(done.len(), 45);
    let mut pct: Vec<f64> = done.iter().map(|(_, t)| t.pct_cpu.unwrap()).collect();
    pct.sort_by(f64::total_cmp);
    let mean = pct.iter().sum::<f64>() / pct.len() as f64;
    assert_eq!(quantile(&pct, 0.5).round(), 104.0);
    assert_eq!(
        (quantile(&pct, 0.25).round(), quantile(&pct, 0.75).round()),
        (94.0, 131.0)
    );
    assert_eq!(mean.round(), 120.0);
    assert_eq!((pct[0].round(), pct[44].round()), (25.0, 511.0));
    assert_eq!(
        format!(
            "{:.2} {:.2} {:.2}",
            quantile(&pct, 0.5) / 100.0,
            mean / 100.0,
            pct[0] / 100.0
        ),
        "1.04 1.20 0.25"
    );
    assert_eq!(format!("{:.1}", pct[44] / 100.0), "5.1");
    let max_rss = done
        .iter()
        .map(|(_, t)| t.peak_rss.unwrap() / GIB)
        .fold(0f64, f64::max);
    assert!(max_rss <= 9.45, "{max_rss}");
    assert_eq!(round(max_rss, 2), 9.45);
}

/// examples/README: "The full-size test failed in 3.22.1, 3.23.0, 3.24.0 and 3.25.0 because an
/// RSEM task hit the 16 h limit"; three of the four logged `Parsed 245000000`, `255000000` and
/// `73000000 entries` last. 3.22.2, between them, passed.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn rsem_timeouts_3_22_1_to_3_25_0() {
    for rev in ["3.22.1", "3.23.0", "3.24.0", "3.25.0"] {
        let run = by_rev(rsem(), rev);
        assert!(run.timed_out(), "{rev}");
        let failed: Vec<&Task> = run
            .tasks_of(RSEM)
            .filter(|t| t.status == "FAILED")
            .collect();
        assert_eq!(failed.len(), 1, "{rev}");
        assert!(failed[0].realtime_s.unwrap() >= 16.0 * 3600.0, "{rev}");
    }
    let parsed: Vec<Option<u64>> = ["3.22.1", "3.23.0", "3.24.0", "3.25.0"]
        .iter()
        .map(|r| by_rev(rsem(), r).last_parsed())
        .collect();
    assert_eq!(
        &parsed[..3],
        &[Some(245_000_000), Some(255_000_000), Some(73_000_000)]
    );
    assert_eq!(parsed[3], None);
    let ok = by_rev(rsem(), "3.22.2");
    assert!(!ok.timed_out());
    assert!(ok.tasks_of(RSEM).all(|t| t.status == "COMPLETED"));
    assert_eq!(ok.tasks_of(RSEM).count(), 8);
    // rsem-threads/README: "Over their 16 h, those tasks averaged about 4.3 k, 4.4 k and 1.3 k entries/s."
    let rates: Vec<f64> = parsed[..3]
        .iter()
        .map(|p| round(p.unwrap() as f64 / 16.0 / 3600.0 / 1000.0, 1))
        .collect();
    assert_eq!(rates, [4.3, 4.4, 1.3]);
}

/// examples/README: "In 3.27.0 the test passed. One task needed three attempts (exit 175), and
/// two tasks ran past the 16 h limit without being stopped (22.6 h and 19.4 h). RSEM is 89% of
/// that $464 run." And 3.22.0: "that one process is 75% of a $294 run at 9% CPU efficiency."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn rsem_3_27_0_and_3_22_0() {
    let run = by_rev(rsem(), "3.27.0");
    let h1: Vec<&Task> = run
        .tasks_of(RSEM)
        .filter(|t| t.name.ends_with("(H1_REP2)"))
        .collect();
    assert_eq!(h1.iter().map(|t| t.attempt.unwrap()).max(), Some(3));
    // The only task in the run that needed a third attempt.
    assert_eq!(run.tasks.iter().filter(|t| t.attempt == Some(3)).count(), 1);
    assert_eq!(h1.last().unwrap().status, "COMPLETED");
    assert!(
        h1.iter().any(|t| t.exit == Some(175)),
        "{:?}",
        h1.iter().map(|t| t.exit).collect::<Vec<_>>()
    );
    let mut over: Vec<f64> = run
        .tasks_of(RSEM)
        .filter_map(|t| t.realtime_s)
        .map(|s| s / 3600.0)
        .filter(|h| *h > 16.0)
        .collect();
    over.sort_by(|a, b| b.total_cmp(a));
    assert_eq!(
        over.iter().map(|h| round(*h, 1)).collect::<Vec<_>>(),
        [22.6, 19.4]
    );
    assert_eq!(
        (run.share(RSEM).round(), run.stats.total_cost.round()),
        (89.0, 464.0)
    );

    let r22 = by_rev(rsem(), "3.22.0");
    let p = r22.process(RSEM);
    assert_eq!(
        (r22.share(RSEM).round(), r22.stats.total_cost.round()),
        (75.0, 294.0)
    );
    assert_eq!(
        (
            (p.cpu_efficiency().unwrap() * 100.0).round(),
            (p.mem_efficiency().unwrap() * 100.0).round()
        ),
        (9.0, 11.0)
    );
    // examples/README table: the RSEM line alone (2 CPU / 12 GiB) is an estimated $185 saving.
    let rec = recommend(&r22.stats, &Rates::SEQERA_COMPUTE, 1.25);
    let rr = rec.iter().find(|x| x.process.ends_with(RSEM)).unwrap();
    assert_eq!(
        (rr.cpus_new, rr.mem_new_gib, rr.saving.round()),
        (2, 12.0, 185.0)
    );
    assert_eq!(rr.time_now_h, 16.0);
}

/// rsem-threads/README: "The megatest tasks read 17.6–20.7× theirs on 44 of 45 completed tasks
/// (their BAM sizes, 20.3–25.8 GB, are in each task's rendered script); the exception,
/// 3.25.0 `K562_REP2`, read 1.3×."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn rsem_read_volume() {
    let mut ratios = Vec::new();
    let mut sizes = Vec::new();
    for run in rsem().iter().filter(|r| release_ge(r.rev(), &[3, 22, 0])) {
        let v = report_json(&run.html).unwrap();
        for rec in v["trace"].as_array().unwrap() {
            let name = rec["name"].as_str().unwrap();
            if !name.contains(RSEM) || rec["status"] != "COMPLETED" {
                continue;
            }
            let script = rec["script"].as_str().unwrap();
            // `[ 22350070361 -gt 1 ] && PAIRED_END_FLAG=...`: the module renders the BAM size in bytes.
            let at = script.find(" -gt 1 ]").expect("BAM size test in script");
            let bam: f64 = script[..at].rsplit(' ').next().unwrap().parse().unwrap();
            let rchar: f64 = rec["rchar"].as_str().unwrap().parse().unwrap();
            sizes.push(bam / 1e9);
            ratios.push((run.rev().to_string(), name.to_string(), rchar / bam));
        }
    }
    assert_eq!(ratios.len(), 45);
    let (lo, hi) = sizes
        .iter()
        .fold((f64::MAX, 0f64), |(l, h), &c| (l.min(c), h.max(c)));
    assert_eq!((round(lo, 1), round(hi, 1)), (20.3, 25.8));
    let (odd, typical): (Vec<_>, Vec<_>) = ratios.into_iter().partition(|(_, _, x)| *x < 10.0);
    assert_eq!(odd.len(), 1);
    assert_eq!(odd[0].0, "3.25.0");
    assert!(odd[0].1.ends_with("(K562_REP2)"));
    assert_eq!(round(odd[0].2, 1), 1.3);
    let (lo, hi) = typical
        .iter()
        .fold((f64::MAX, 0f64), |(l, h), x| (l.min(x.2), h.max(x.2)));
    assert_eq!((round(lo, 1), round(hi, 1)), (17.6, 20.7));
}

/// examples/README: "RSEM's share jumped from 31–48% (3.1–3.10.1) to 64–68% (3.11.1–3.17.0)."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn rsem_share_history() {
    let r = rsem();
    let band = |from: &str, to: &str| {
        let s: Vec<f64> = r[pos(r, from)..=pos(r, to)]
            .iter()
            .map(|x| x.share(RSEM).round())
            .collect();
        (
            s.iter().cloned().fold(f64::MAX, f64::min),
            s.iter().cloned().fold(0.0, f64::max),
        )
    };
    assert_eq!(band("3.1", "3.10.1"), (31.0, 48.0));
    assert_eq!(band("3.11.1", "3.17.0"), (64.0, 68.0));
}

/// examples/README "dev-branch runs": of 19 `star_rsem` runs, 7 stopped on the RSEM 16 h
/// timeout; six end on `Parsed N entries`, one on `515000000 alignment lines are loaded!`.
/// Three June 2025 runs under `results-dev/pipeline_info/` also timed out on `Parsed N entries`.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn dev_branch_timeouts() {
    let reports = |dir: &str| -> Vec<Run> {
        manifest(&format!("data/rnaseq/results-dev/{dir}/execution_report_"))
            .into_iter()
            .map(Run::load)
            .collect()
    };
    let rsem_timeout = |r: &Run| {
        r.timed_out()
            && r.tasks_of(RSEM)
                .any(|t| t.status == "FAILED" && t.realtime_s.unwrap_or(0.0) >= 16.0 * 3600.0)
    };
    let dev = reports("aligner_star_rsem/pipeline_info");
    assert_eq!(dev.len(), 19);
    let hit: Vec<&Run> = dev.iter().filter(|r| rsem_timeout(r)).collect();
    assert_eq!(hit.len(), 7);
    assert_eq!(hit.iter().filter(|r| r.last_parsed().is_some()).count(), 6);
    assert_eq!(
        hit.iter()
            .filter(|r| r.last_output_line() == Some("515000000 alignment lines are loaded!"))
            .count(),
        1
    );

    let june: Vec<Run> = reports("pipeline_info")
        .into_iter()
        .filter(|r| {
            r.meta
                .started
                .as_deref()
                .is_some_and(|s| s.contains("-Jun-2025"))
        })
        .filter(rsem_timeout)
        .collect();
    assert_eq!(june.len(), 3);
    assert!(june.iter().all(|r| r.last_parsed().is_some()));
}

// ---------------------------------------------------------------------------------------------
// examples/README points 1 and 2 (star_salmon)

/// "Unused allocation has stayed at 62–68% of metered allocated cost in 28 of the 30
/// `star_salmon` releases. The exceptions are 3.5 (81%, a `-resume` run) and 3.6 (73%, 13
/// failed attempts). Over the same period the allocated cost fell from $139 (3.1) to $84
/// (3.11.1) and $60 (3.26.0)."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn salmon_unused_share() {
    let s = salmon();
    let outside: Vec<(&str, f64)> = s
        .iter()
        .map(|r| (r.rev(), r.unused_pct().round()))
        .filter(|(_, u)| !(62.0..=68.0).contains(u))
        .collect();
    assert_eq!(outside, [("3.5", 81.0), ("3.6", 73.0)]);
    let f36: usize = by_rev(s, "3.6")
        .stats
        .processes
        .iter()
        .map(|p| p.failed_tasks)
        .sum();
    assert_eq!(f36, 13);
    assert!(by_rev(s, "3.5").tasks.iter().any(|t| t.status == "CACHED"));
    let c = |rev| by_rev(s, rev).stats.total_cost.round();
    assert_eq!((c("3.1"), c("3.11.1"), c("3.26.0")), (139.0, 84.0, 60.0));
}

/// "3.25.0 → 3.26.0: TRIMGALORE ... from 12 CPU / 72 GiB to 8 CPU / 1 GiB. Its run time fell
/// from 4.38 h to 1.62 h, and its cost from $13.15 to $1.34. STAR went from $23.86 to $16.16.
/// The two differences sum to $19.51 of the $19.65 net drop. Other processes moved by up to
/// about ±$3."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn salmon_3_25_to_3_26() {
    let (a, b) = (by_rev(salmon(), "3.25.0"), by_rev(salmon(), "3.26.0"));
    let req = |r: &Run| {
        let p = r.process("TRIMGALORE");
        (p.max_cpus_requested, p.max_mem_requested_gib)
    };
    assert_eq!((req(a), req(b)), ((12.0, 72.0), (8.0, 1.0)));
    let (ta, tb) = (a.process("TRIMGALORE"), b.process("TRIMGALORE"));
    assert_eq!(
        (round(ta.realtime_h, 2), round(tb.realtime_h, 2)),
        (4.38, 1.62)
    );
    assert_eq!(format!("{:.2} {:.2}", ta.cost, tb.cost), "13.15 1.34");
    let star = |r: &Run| {
        r.stats
            .processes
            .iter()
            .filter(|p| p.process.contains(":STAR_ALIGN"))
            .map(|p| p.cost)
            .sum::<f64>()
    };
    assert_eq!(format!("{:.2} {:.2}", star(a), star(b)), "23.86 16.16");
    let two = (ta.cost - tb.cost) + (star(a) - star(b));
    let net = a.stats.total_cost - b.stats.total_cost;
    assert_eq!(format!("{two:.2} {net:.2}"), "19.51 19.65");
    // Every other process, matched on its last name component.
    let last = |p: &str| p.rsplit(':').next().unwrap().to_string();
    let names: BTreeSet<String> = a
        .stats
        .processes
        .iter()
        .chain(&b.stats.processes)
        .map(|p| last(&p.process))
        .collect();
    let cost = |r: &Run, n: &str| {
        r.stats
            .processes
            .iter()
            .filter(|p| last(&p.process) == n)
            .map(|p| p.cost)
            .sum::<f64>()
    };
    let others = names
        .iter()
        .filter(|n| *n != "TRIMGALORE" && !n.starts_with("STAR_ALIGN"))
        .map(|n| (cost(b, n) - cost(a, n)).abs())
        .fold(0.0, f64::max);
    assert_eq!(format!("{others:.2}"), "3.19");
    let dup = cost(b, "DUPRADAR") - cost(a, "DUPRADAR");
    assert_eq!(
        format!("{dup:.2}"),
        "-3.19",
        "the largest other move is DUPRADAR, down"
    );
}

/// "`QUALIMAP_RNASEQ` requests 6 CPUs / 36 GiB in every release, and no task of it averaged
/// more than 1.08 cores. In 3.6, two tasks that failed their first attempt were retried at
/// 12 / 72." Ranks and shares per release follow.
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn salmon_qualimap() {
    let s = salmon();
    const Q: &str = "QUALIMAP_RNASEQ";
    let mut max_cores = 0f64;
    for run in s {
        for t in run.tasks_of(Q) {
            if t.attempt == Some(1) {
                assert_eq!(
                    (t.cpus, t.memory.map(|m| m / GIB)),
                    (Some(6.0), Some(36.0)),
                    "{}",
                    run.rev()
                );
            }
            if let Some(p) = t.pct_cpu {
                max_cores = max_cores.max(p / 100.0);
            }
        }
    }
    assert_eq!(round(max_cores, 2), 1.08);
    let retried: Vec<&Task> = by_rev(s, "3.6")
        .tasks_of(Q)
        .filter(|t| t.attempt == Some(2))
        .collect();
    assert_eq!(retried.len(), 2);
    assert!(retried
        .iter()
        .all(|t| (t.cpus, t.memory.map(|m| m / GIB)) == (Some(12.0), Some(72.0))));

    let rank = |r: &Run| {
        1 + r
            .stats
            .processes
            .iter()
            .position(|p| p.process.ends_with(Q))
            .unwrap()
    };
    for (from, to, want) in [
        ("3.1", "3.4", 3),
        ("3.7", "3.8.1", 3),
        ("3.5", "3.5", 4),
        ("3.6", "3.6", 2),
        ("3.9", "3.18.0", 2),
    ] {
        for r in &s[pos(s, from)..=pos(s, to)] {
            assert_eq!(rank(r), want, "{}", r.rev());
        }
    }
    let mut runs: Vec<&Run> = vec![by_rev(s, "3.6")];
    runs.extend(&s[pos(s, "3.9")..=pos(s, "3.18.0")]);
    let shares: Vec<f64> = runs.iter().map(|r| r.share(Q).round()).collect();
    let effs: Vec<f64> = runs
        .iter()
        .map(|r| (r.process(Q).cpu_efficiency().unwrap() * 100.0).round())
        .collect();
    let mm = |v: &[f64]| {
        (
            v.iter().cloned().fold(f64::MAX, f64::min),
            v.iter().cloned().fold(0.0, f64::max),
        )
    };
    assert_eq!(mm(&shares), (18.0, 23.0));
    assert_eq!(mm(&effs), (13.0, 17.0));
    let (a, b) = (by_rev(s, "3.18.0"), by_rev(s, "3.22.0"));
    assert_eq!(b.share(Q).round(), 9.0);
    assert_eq!(
        (
            round(a.process(Q).realtime_h, 1),
            round(b.process(Q).realtime_h, 1)
        ),
        (11.6, 4.6)
    );
    let st = |r: &Run| r.process("STAR_ALIGN_IGENOMES").cost;
    assert_eq!(format!("{:.2} {:.2}", st(a), st(b)), "19.24 24.43");
    assert_eq!(b.share("STAR_ALIGN_IGENOMES").round(), 31.0);
}

/// examples/README: "The 3.15.1 report is a `-resume` run: 195 of 271 tasks are cached ...
/// 3.5's successful runs on both routes are also `-resume` runs."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn resume_runs() {
    let r = by_rev(rsem(), "3.15.1");
    let cached = r.tasks.iter().filter(|t| t.status == "CACHED").count();
    assert_eq!((cached, r.tasks.len()), (195, 271));
    for runs in [salmon(), rsem()] {
        assert!(by_rev(runs, "3.5")
            .tasks
            .iter()
            .any(|t| t.status == "CACHED"));
    }
}

/// README "What it reads" / examples: the two June 2025 3.19.0 runs carry no usage metrics, and
/// "both aligner runs failed on a QUALIMAP task".
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn unmetered_3_19_0() {
    let mut n = 0;
    for p in
        manifest("data/rnaseq/results-0bb032c1e3b1e1ff0b0a72192b9118fdb5062489/pipeline_info/execution_report_")
    {
        let run = Run::load(p);
        n += 1;
        assert_eq!(run.rev(), "3.19.0");
        assert_eq!(run.meta.nextflow_version.as_deref(), Some("25.04.2"));
        assert_eq!(run.stats.tasks_without_metrics, run.stats.tasks);
        let failed: Vec<&Task> = run.tasks.iter().filter(|t| t.status == "FAILED").collect();
        assert!(
            !failed.is_empty() && failed.iter().all(|t| t.process.contains("QUALIMAP")),
            "{}",
            run.path.display()
        );
    }
    assert_eq!(n, 2);
}

/// examples/README point 1: "3.10.1 → 3.11.1: profile `test_full,aws_tower` → `test_full_aws`";
/// "Between 3.18.0 and 3.22.0: the profile changed again (`test_full_aws` → `test_full`, already
/// so in the 3.19.0 runs), with Nextflow 24.10 → 25.04." Point 3: across 3.10.1 → 3.11.1 "RSEM's
/// script, arguments and base image are identical; from 3.11.1 the image is served through Wave,
/// and Nextflow went from 22.10.4 to 23.03.0-edge."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn launch_changes() {
    for runs in [salmon(), rsem()] {
        let m = |rev| {
            let r = by_rev(runs, rev);
            (
                r.meta.profile.clone().unwrap(),
                r.meta.nextflow_version.clone().unwrap(),
            )
        };
        assert_eq!(
            m("3.10.1"),
            ("test_full,aws_tower".into(), "22.10.4".into())
        );
        assert_eq!(m("3.11.1"), ("test_full_aws".into(), "23.03.0-edge".into()));
        assert_eq!(m("3.18.0"), ("test_full_aws".into(), "24.10.3".into()));
        assert_eq!(m("3.22.0"), ("test_full".into(), "25.04.8".into()));
    }
    for p in
        manifest("data/rnaseq/results-0bb032c1e3b1e1ff0b0a72192b9118fdb5062489/pipeline_info/execution_report_")
    {
        assert_eq!(Run::load(p).meta.profile.as_deref(), Some("test_full"));
    }
    // RSEM task: script with the sample name masked, and container, one task per release.
    let rsem_task = |rev| {
        let run = by_rev(rsem(), rev);
        let v = report_json(&run.html).unwrap();
        let rec = v["trace"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str().unwrap().contains(RSEM))
            .unwrap()
            .clone();
        let tag = rec["tag"].as_str().unwrap().to_string();
        (
            rec["script"].as_str().unwrap().replace(&tag, "<S>"),
            rec["container"].as_str().unwrap().to_string(),
        )
    };
    let (s10, c10) = rsem_task("3.10.1");
    let (s11, c11) = rsem_task("3.11.1");
    assert_eq!(s10, s11);
    let image = |c: &str| c.rsplit_once("/biocontainers/").unwrap().1.to_string();
    assert!(c10.starts_with("quay.io/biocontainers/"), "{c10}");
    assert!(c11.starts_with("wave.seqera.io/"), "{c11}");
    assert_eq!(image(&c10), image(&c11));
}

/// examples/README "Which runs": "for each release tag and route, the last successful run stored
/// under that tag's own `results-<commit>/` prefix. On `star_rsem`, where a release has none, the
/// last run in which `RSEM_CALCULATEEXPRESSION` failed." A task "failed" here has status FAILED;
/// tasks ABORTED because another task failed do not count. "Left out": 3.11.0 (no run qualifies
/// on either route), 3.13.0 `star_salmon`, 3.26.0 `star_rsem` ("both of its runs stopped early
/// for reasons unrelated to RSEM").
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn corpus_selection_rule() {
    let ok = |r: &Run| !r.html.contains("completed unsuccessfully");
    let rsem_failed = |r: &Run| r.tasks_of(RSEM).any(|t| t.status == "FAILED");
    let siblings = |r: &Run| -> Vec<Run> {
        let dir = r.path.parent().unwrap().strip_prefix(root()).unwrap();
        manifest(&format!("{}/execution_report_", dir.display()))
            .into_iter()
            .map(Run::load)
            .collect()
    };
    let last_by_name = |runs: &[Run], pick: &dyn Fn(&Run) -> bool| -> Option<PathBuf> {
        runs.iter()
            .filter(|r| pick(r))
            .map(|r| r.path.clone())
            .max()
    };
    for (runs, route_rsem) in [(salmon(), false), (rsem(), true)] {
        for run in runs {
            let sib = siblings(run);
            let want = match last_by_name(&sib, &ok) {
                Some(p) => p,
                None => {
                    assert!(route_rsem, "{}: no successful run", run.path.display());
                    last_by_name(&sib, &rsem_failed).expect("an RSEM failure")
                }
            };
            assert_eq!(
                run.path,
                want,
                "{} is not the run the rule picks",
                run.rev()
            );
        }
    }
    let failed_listed: Vec<&str> = rsem().iter().filter(|r| !ok(r)).map(Run::rev).collect();
    assert_eq!(failed_listed, ["3.22.1", "3.23.0", "3.24.0", "3.25.0"]);

    let left_out = |prefix: &str, n: usize, rsem_route: bool| {
        let runs: Vec<Run> = manifest(prefix).into_iter().map(Run::load).collect();
        assert_eq!(runs.len(), n, "{prefix}");
        assert!(runs.iter().all(|r| !ok(r)), "{prefix}: a run succeeded");
        if rsem_route {
            assert!(
                runs.iter().all(|r| !rsem_failed(r)),
                "{prefix}: an RSEM task failed"
            );
        }
    };
    let p3110 = "data/rnaseq/results-48fb9b4ea640f029f48c79283217d0f20661d38e";
    left_out(
        &format!("{p3110}/aligner_star_salmon/pipeline_info/execution_report_"),
        1,
        false,
    );
    left_out(
        &format!("{p3110}/aligner_star_rsem/pipeline_info/execution_report_"),
        1,
        true,
    );
    left_out("data/rnaseq/results-14f9d26444e08da7b51ddcb1b8c4e0703edde375/aligner_star_salmon/pipeline_info/execution_report_", 4, false);
    left_out("data/rnaseq/results-e7ca46272c8f9d5ceee3f71759f4ba551d3217a4/aligner_star_rsem/pipeline_info/execution_report_", 2, true);
}

/// README "What it reads": "the June 2025 ones (Nextflow 25.04.2, Fusion 2.4) are unmetered; in
/// every other run of the corpus, every completed task carries usage metrics (the tasks without
/// them, at most 13 in a run, are failed or aborted attempts)."
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn metering_outside_3_19_0() {
    let mut most = 0;
    for run in salmon().iter().chain(rsem()) {
        let unmetered: Vec<&Task> = run
            .tasks
            .iter()
            .filter(|t| t.pct_cpu.is_none() && t.peak_rss.is_none())
            .collect();
        assert!(
            unmetered.iter().all(|t| !t.succeeded()),
            "{}: a completed task without metrics",
            run.rev()
        );
        most = most.max(unmetered.len());
    }
    assert_eq!(most, 13);
    for p in manifest("data/rnaseq/results-0bb032c1e3b1e1ff0b0a72192b9118fdb5062489/pipeline_info/execution_report_") {
        assert_eq!(Run::load(p).meta.fusion.as_deref(), Some("true, version 2.4"));
    }
}

/// README "Examples": `star_salmon` "3.1 (May 2021) → 3.26.0 (May 2026)"; the 3.22.0 `star_rsem`
/// run is the first after STAR moved out of the RSEM task (3.21.0 has no report); four of the
/// five releases 3.22.1–3.25.0 failed. examples/README: the 3.22.0 fragment "keeps the 16 h
/// limit"; the June 2025 `dev` runs are "from before #1604, when the task still ran STAR".
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn release_span_and_rsem_history() {
    let s = salmon();
    assert!(s[0].meta.started.as_deref().unwrap().contains("-May-2021"));
    assert!(s[29].meta.started.as_deref().unwrap().contains("-May-2026"));
    let r = rsem();
    let first_split = r.iter().find(|x| release_ge(x.rev(), &[3, 21, 0])).unwrap();
    assert_eq!(first_split.rev(), "3.22.0");
    let span: Vec<&Run> = r
        .iter()
        .filter(|x| release_ge(x.rev(), &[3, 22, 1]) && !release_ge(x.rev(), &[3, 25, 1]))
        .collect();
    assert_eq!(span.len(), 5);
    assert_eq!(span.iter().filter(|x| x.timed_out()).count(), 4);
    let r22 = by_rev(r, "3.22.0");
    let rec = recommend(&r22.stats, &Rates::SEQERA_COMPUTE, 1.25);
    let rr = rec.iter().find(|x| x.process.ends_with(RSEM)).unwrap();
    assert_eq!(rr.time_new_h, 16.0);
    // Before #1604 the RSEM task aligned with STAR itself (`--star`); from 3.22.0 it reads a BAM.
    let june: Vec<Run> = manifest("data/rnaseq/results-dev/pipeline_info/execution_report_2025-06")
        .into_iter()
        .map(Run::load)
        .filter(|x| x.timed_out())
        .collect();
    assert!(june.iter().all(|x| x.html.contains("--star ")));
    assert!(!r22.html.contains("--star "));
}

/// examples/README: "Reports from 3.13.x show the revision as `master`; the tables label them
/// `master@<commit>` and place them by start date" (between 3.12.0 and 3.14.0).
#[test]
#[ignore = "needs data/: scripts/pull-megatests.sh --corpus"]
fn master_runs_in_compare() {
    for (list, n) in [
        ("rnaseq-star_salmon-releases.files", 2),
        ("rnaseq-star_rsem-releases.files", 3),
    ] {
        let text = std::fs::read_to_string(root().join("examples").join(list)).unwrap();
        let paths: Vec<PathBuf> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| root().join(l))
            .collect();
        let runs = nf_audit::compare::load_runs(&paths, &Rates::SEQERA_COMPUTE).unwrap();
        let labels: Vec<&str> = runs.iter().map(|r| r.label.as_str()).collect();
        let masters: Vec<usize> = (0..labels.len())
            .filter(|&i| labels[i].starts_with("master@"))
            .collect();
        assert_eq!(masters.len(), n, "{labels:?}");
        let at = |rev: &str| {
            labels
                .iter()
                .position(|l| l.starts_with(&format!("{rev} ")))
                .unwrap()
        };
        assert!(
            masters
                .iter()
                .all(|&i| i > at("3.12.0") && i < at("3.14.0")),
            "{labels:?}"
        );
        assert!(runs
            .iter()
            .filter(|r| r.label.starts_with("master@"))
            .all(|r| r.meta.revision.as_deref() == Some("master")));
    }
}
