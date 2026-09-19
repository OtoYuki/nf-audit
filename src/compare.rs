//! Multi-run comparison: the same pipeline across releases (or the same release across
//! executors), one line per run and a process-by-run cost-share matrix.

use crate::analysis::{analyse, Rates, RunStats};
use crate::input::{read_report_with_meta, RunMeta};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub struct Run {
    pub path: PathBuf,
    pub meta: RunMeta,
    pub stats: RunStats,
    pub label: String,
    /// Sort key: numeric release first, then everything else by start date.
    key: (Vec<u32>, String),
}

pub fn load_runs(paths: &[PathBuf], rates: &Rates) -> Result<Vec<Run>> {
    let mut runs = Vec::with_capacity(paths.len());
    for p in paths {
        let (tasks, meta) =
            read_report_with_meta(p).with_context(|| format!("loading {}", p.display()))?;
        let stats = analyse(&tasks, rates);
        let aligner = aligner_hint(&stats, p);
        let date = start_date(&meta);
        let rev = meta.revision.clone().unwrap_or_else(|| "?".into());
        let label = [Some(rev.clone()), aligner, Some(date.clone())]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let key = (release_key(&rev), date);
        runs.push(Run {
            path: p.clone(),
            meta,
            stats,
            label,
            key,
        });
    }
    runs.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(runs)
}

/// `3.15.1` -> [3, 15, 1]; anything else -> [] so it sorts first-by-empty, then by date.
fn release_key(rev: &str) -> Vec<u32> {
    let parts: Vec<u32> = rev
        .split('.')
        .map(|x| x.parse::<u32>())
        .collect::<std::result::Result<_, _>>()
        .unwrap_or_default();
    if parts.is_empty() {
        vec![u32::MAX]
    } else {
        parts
    }
}

/// `16-Sep-2024 16:33:27` -> `2024-09-16`; falls back to the raw string.
fn start_date(meta: &RunMeta) -> String {
    let Some(s) = &meta.started else {
        return String::new();
    };
    let mut it = s.split_whitespace().next().unwrap_or("").split('-');
    let (Some(d), Some(mon), Some(y)) = (it.next(), it.next(), it.next()) else {
        return s.clone();
    };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    match MONTHS.iter().position(|m| m.eq_ignore_ascii_case(mon)) {
        Some(i) => format!("{y}-{:02}-{:0>2}", i + 1, d),
        None => s.clone(),
    }
}

/// Which aligner branch ran, read off the processes present. nf-core/rnaseq megatests run
/// `star_salmon` and `star_rsem` as separate runs of the same revision.
fn aligner_hint(stats: &RunStats, path: &Path) -> Option<String> {
    let has = |suffix: &str| stats.processes.iter().any(|p| p.process.ends_with(suffix));
    if has("RSEM_CALCULATEEXPRESSION") {
        Some("rsem".into())
    } else if has("HISAT2_ALIGN") {
        Some("hisat2".into())
    } else if has("STAR_ALIGN") || has("STAR_ALIGN_IGENOMES") {
        Some("salmon".into())
    } else {
        let p = path.to_string_lossy();
        ["star_salmon", "star_rsem", "hisat2"]
            .iter()
            .find(|a| p.contains(*a))
            .map(|a| a.trim_start_matches("star_").to_string())
    }
}

pub fn render_markdown(runs: &[Run], r: &Rates, top: usize) -> String {
    let mut s = String::new();
    s.push_str("# nf-audit compare\n\n");
    s.push_str(&format!(
        "- rates: `{}` (${:.4}/CPU-h, ${:.4}/GiB-h)\n- runs: {}\n\n",
        r.name,
        r.cpu_hour,
        r.gib_hour,
        runs.len()
    ));

    s.push_str("## Runs\n\n");
    s.push_str("| run | nextflow | fusion | tasks | failed | wall | CPU-h req | GiB-h req | cost | metered | unused | top process |\n");
    s.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for run in runs {
        let st = &run.stats;
        let names = display_names(st);
        let failed: usize = st.processes.iter().map(|p| p.failed_tasks).sum();
        let metered = st.tasks.saturating_sub(st.tasks_without_metrics);
        let metered_cost = st.cpu_h_req_metered * r.cpu_hour + st.gib_h_req_metered * r.gib_hour;
        let unused = if metered_cost > 0.0 {
            format!("{:.0}%", st.total_waste / metered_cost * 100.0)
        } else {
            "n/a".into()
        };
        let top_p = st
            .processes
            .first()
            .map(|p| {
                format!(
                    "{} {:.0}%",
                    names
                        .get(p.process.as_str())
                        .map(String::as_str)
                        .unwrap_or(&p.process),
                    if st.total_cost > 0.0 {
                        p.cost / st.total_cost * 100.0
                    } else {
                        0.0
                    }
                )
            })
            .unwrap_or_default();
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:.0} | {:.0} | ${:.2} | {}/{} | {} | {} |\n",
            run.label,
            run.meta.nextflow_version.as_deref().unwrap_or("?"),
            run.meta
                .fusion
                .as_deref()
                .map(|f| if f.starts_with("true") {
                    f.trim_start_matches("true, version ").to_string()
                } else {
                    "no".into()
                })
                .unwrap_or_else(|| "?".into()),
            st.tasks,
            failed,
            run.meta.duration.as_deref().unwrap_or("?"),
            st.cpu_h_requested,
            st.gib_h_requested,
            st.total_cost,
            metered,
            st.tasks,
            unused,
            top_p
        ));
    }
    s.push('\n');
    s.push_str("Sources:\n\n");
    for run in runs {
        s.push_str(&format!("- {} — `{}`\n", run.label, run.path.display()));
    }
    s.push('\n');

    // Process x run share matrix over the processes that cost the most summed across runs.
    let mut total_by_proc: BTreeMap<String, f64> = BTreeMap::new();
    let mut share: HashMap<(String, usize), f64> = HashMap::new();
    for (i, run) in runs.iter().enumerate() {
        let names = display_names(&run.stats);
        for p in &run.stats.processes {
            let name = names
                .get(p.process.as_str())
                .cloned()
                .unwrap_or_else(|| p.process.clone());
            *total_by_proc.entry(name.clone()).or_default() += p.cost;
            let sh = if run.stats.total_cost > 0.0 {
                p.cost / run.stats.total_cost * 100.0
            } else {
                0.0
            };
            *share.entry((name, i)).or_default() += sh;
        }
    }
    let mut procs: Vec<(String, f64)> = total_by_proc.into_iter().collect();
    procs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(top);

    s.push_str(&format!(
        "## Cost share by process (top {top}, % of each run's cost)\n\n"
    ));
    s.push_str("| process |");
    for run in runs {
        s.push_str(&format!(" {} |", run.label));
    }
    s.push_str("\n|---|");
    for _ in runs {
        s.push_str("---:|");
    }
    s.push('\n');
    for (name, _) in &procs {
        s.push_str(&format!("| {name} |"));
        for i in 0..runs.len() {
            match share.get(&(name.clone(), i)) {
                Some(v) => s.push_str(&format!(" {v:.0} |")),
                None => s.push_str(" · |"),
            }
        }
        s.push('\n');
    }
    s.push_str("\n`·` = process absent from that run.\n");
    s
}

fn display_names(st: &RunStats) -> HashMap<&str, String> {
    crate::display_names(st.processes.iter().map(|p| p.process.as_str()))
}
