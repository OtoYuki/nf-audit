//! nf-audit: where the CPU-hours and dollars of a Nextflow run actually go.
//!
//! Reads the `execution_trace_*.txt` and/or `execution_report_*.html` that every Nextflow run
//! writes, aggregates per process, prices the run under a chosen rate table, quantifies
//! over-allocation and retry waste, and emits a right-sized `nextflow.config` fragment.
//!
//! Works offline, on any executor, on runs you did not execute through Seqera Platform.

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use nf_audit::analysis::{self, analyse, fmt_time_h, recommend, Rates, RunStats};
use nf_audit::{compare, display_names, input};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "nf-audit",
    version,
    about = "Cost and right-sizing audit of Nextflow runs from trace and report files"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Analyse one run and print a Markdown report to stdout.
    Analyze {
        /// execution_trace_*.txt (TSV). Optional if --report is given.
        #[arg(long)]
        trace: Option<PathBuf>,
        /// execution_report_*.html. Carries requested cpus/memory/time, which the default trace lacks.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Rate preset: seqera-compute | aws-m5-ondemand | aws-m5-spot
        #[arg(long, default_value = "seqera-compute")]
        rates: String,
        /// Override price per CPU-hour (USD).
        #[arg(long, value_parser = price_arg)]
        cpu_hour: Option<f64>,
        /// Override price per GiB-hour (USD).
        #[arg(long, value_parser = price_arg)]
        gib_hour: Option<f64>,
        /// Safety margin applied to observed peaks when recommending new requests (at least 1).
        #[arg(long, default_value_t = 1.25, value_parser = margin_arg)]
        margin: f64,
        /// Write a right-sized nextflow.config fragment here.
        #[arg(long)]
        config_out: Option<PathBuf>,
        /// Show at most this many processes in tables.
        #[arg(long, default_value_t = 25)]
        top: usize,
        /// Output results as JSON instead of Markdown.
        #[arg(long)]
        json: bool,
    },
    /// Compare several runs (one report each): per-run totals and a process-by-run cost-share matrix.
    Compare {
        /// execution_report_*.html files, one per run.
        #[arg(required = true)]
        reports: Vec<PathBuf>,
        /// Rate preset: seqera-compute | aws-m5-ondemand | aws-m5-spot
        #[arg(long, default_value = "seqera-compute")]
        rates: String,
        /// Override price per CPU-hour (USD).
        #[arg(long, value_parser = price_arg)]
        cpu_hour: Option<f64>,
        /// Override price per GiB-hour (USD).
        #[arg(long, value_parser = price_arg)]
        gib_hour: Option<f64>,
        /// Rows in the process-share matrix.
        #[arg(long, default_value_t = 15)]
        top: usize,
    },
    /// Print the columns found in a trace file and how many tasks it holds. Use this first.
    Inspect {
        path: PathBuf,
        /// Output inspection facts as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn margin_arg(s: &str) -> std::result::Result<f64, String> {
    match s.parse::<f64>() {
        Ok(m) if m.is_finite() && m >= 1.0 => Ok(m),
        _ => Err(
            "must be a number >= 1 (a margin below 1 would size below the observed peak)".into(),
        ),
    }
}

fn price_arg(s: &str) -> std::result::Result<f64, String> {
    match s.parse::<f64>() {
        Ok(p) if p.is_finite() && p >= 0.0 => Ok(p),
        _ => Err("must be a non-negative number of USD".into()),
    }
}

#[derive(serde::Serialize)]
struct AnalyzeJson<'a> {
    rates: &'a Rates,
    run: &'a RunStats,
    meta: &'a input::RunMeta,
    recommendations: &'a [analysis::Recommendation],
    /// Processes with requests that were not sized (too few tasks completed with both metrics).
    not_sized: Vec<&'a str>,
}

#[derive(serde::Serialize)]
struct InspectJson {
    file: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<input::RunMeta>,
    tasks: usize,
    tasks_with_requests: usize,
    tasks_with_metrics: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    columns: Vec<String>,
}

/// `println!` / `print!` that return an error instead of panicking when stdout is closed
/// (`nf-audit inspect report.html | head -1`).
macro_rules! out {
    ($($t:tt)*) => { writeln!(std::io::stdout(), $($t)*)? };
}
macro_rules! outp {
    ($($t:tt)*) => { write!(std::io::stdout(), $($t)*)? };
}

fn main() -> Result<()> {
    match run() {
        Err(e)
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe) =>
        {
            Ok(())
        }
        r => r,
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Inspect { path, json } => inspect(&path, json),
        Cmd::Compare {
            reports,
            rates,
            cpu_hour,
            gib_hour,
            top,
        } => {
            let mut r =
                Rates::preset(&rates).ok_or_else(|| anyhow!("unknown rate preset `{rates}`"))?;
            if let Some(c) = cpu_hour {
                r.cpu_hour = c;
            }
            if let Some(g) = gib_hour {
                r.gib_hour = g;
            }
            let runs = compare::load_runs(&reports, &r)?;
            outp!("{}", compare::render_markdown(&runs, &r, top));
            Ok(())
        }
        Cmd::Analyze {
            trace,
            report,
            rates,
            cpu_hour,
            gib_hour,
            margin,
            config_out,
            top,
            json,
        } => {
            let mut r =
                Rates::preset(&rates).ok_or_else(|| anyhow!("unknown rate preset `{rates}`"))?;
            if let Some(c) = cpu_hour {
                r.cpu_hour = c;
            }
            if let Some(g) = gib_hour {
                r.gib_hour = g;
            }
            let t = match &trace {
                Some(p) => input::read_trace(p)?,
                None => Vec::new(),
            };
            let (rep, meta) = match &report {
                Some(p) => input::read_report_with_meta(p)?,
                None => (Vec::new(), input::RunMeta::default()),
            };
            if trace.is_none() && report.is_none() {
                return Err(anyhow!("give --trace and/or --report"));
            }
            if t.is_empty() && rep.is_empty() {
                return Err(anyhow!(
                    "no tasks in the given file(s): the run recorded none"
                ));
            }
            let tasks = input::merge(t, rep);
            let run = analyse(&tasks, &r);
            let recs = recommend(&run, &r, margin);
            // Written before stdout, so a closed pipe cannot skip it.
            if let Some(out) = config_out {
                fs::write(&out, analysis::render_config(&recs))?;
                eprintln!("wrote {}", out.display());
            }
            if json {
                let payload = AnalyzeJson {
                    rates: &r,
                    run: &run,
                    meta: &meta,
                    recommendations: &recs,
                    not_sized: analysis::not_sized(&run),
                };
                out!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                outp!(
                    "{}",
                    render_markdown(
                        &run,
                        &r,
                        &recs,
                        top,
                        trace.as_deref().map(|p| p.display().to_string()),
                        report.as_deref().map(|p| p.display().to_string()),
                        &meta
                    )
                );
            }
            Ok(())
        }
    }
}

fn inspect(path: &std::path::Path, as_json: bool) -> Result<()> {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.ends_with(".html") {
        let (tasks, meta) = input::read_report_with_meta(path)?;
        let with_req = tasks
            .iter()
            .filter(|t| t.cpus.is_some() && t.memory.is_some())
            .count();
        let with_metrics = tasks
            .iter()
            .filter(|t| t.pct_cpu.is_some() || t.peak_rss.is_some())
            .count();
        if as_json {
            let info = InspectJson {
                file: path.display().to_string(),
                kind: "report",
                meta: Some(meta),
                tasks: tasks.len(),
                tasks_with_requests: with_req,
                tasks_with_metrics: with_metrics,
                columns: Vec::new(),
            };
            out!("{}", serde_json::to_string_pretty(&info)?);
            return Ok(());
        }
        if let Some(line) = meta.summary_line() {
            out!("run: {line}");
        }
        if let Some(c) = &meta.command {
            out!("command: {c}");
        }
        if let Some(w) = &meta.wave {
            out!("wave: {w}");
        }
        if let Some(c) = &meta.cpu_hours {
            out!("nextflow's own CPU-hours figure: {c}");
        }
        out!("report: {} tasks, {} with requested cpus+memory, {} with usage metrics (%cpu/peak_rss)", tasks.len(), with_req, with_metrics);
        if with_metrics == 0 && !tasks.is_empty() {
            out!("note: no task has usage metrics, so this run can be priced but not right-sized.");
        }
        if let Some(t) = tasks.first() {
            out!("first task: {} status={} cpus={:?} memory={:?} time={:?} realtime={:?}s %cpu={:?} peak_rss={:?}",
                t.name, t.status, t.cpus, t.memory, t.time_s, t.realtime_s, t.pct_cpu, t.peak_rss);
        }
    } else {
        let text = fs::read_to_string(path)?;
        let header = text.lines().next().unwrap_or("");
        let cols: Vec<&str> = header.split('\t').map(str::trim).collect();
        // Same task list `analyze --trace` uses: a hash the trace logs twice is one task.
        let tasks = input::merge(input::read_trace(path)?, Vec::new());
        let tasks_count = tasks.len();
        let with_req = tasks
            .iter()
            .filter(|t| t.cpus.is_some() && t.memory.is_some())
            .count();
        let with_metrics = tasks
            .iter()
            .filter(|t| t.pct_cpu.is_some() || t.peak_rss.is_some())
            .count();
        if as_json {
            let info = InspectJson {
                file: path.display().to_string(),
                kind: "trace",
                meta: None,
                tasks: tasks_count,
                tasks_with_requests: with_req,
                tasks_with_metrics: with_metrics,
                columns: cols.iter().map(|s| s.to_string()).collect(),
            };
            out!("{}", serde_json::to_string_pretty(&info)?);
            return Ok(());
        }
        out!(
            "trace: {} columns, {} tasks, {} with requested cpus+memory, {} with usage metrics (%cpu/peak_rss)",
            cols.len(),
            tasks_count,
            with_req,
            with_metrics
        );
        out!("columns: {}", cols.join(", "));
        let has = |c: &str| cols.contains(&c);
        if !(has("cpus") && has("memory")) {
            out!("note: no `cpus`/`memory` columns. Pass the matching execution_report_*.html with --report to get requested resources, or add `trace.fields` to nextflow.config for future runs.");
        }
    }
    Ok(())
}

fn money(x: f64) -> String {
    format!("${:.2}", x)
}
fn pct(x: Option<f64>) -> String {
    match x {
        Some(v) => format!("{:.0}%", v * 100.0),
        None => "n/a".into(),
    }
}

fn render_markdown(
    run: &RunStats,
    r: &Rates,
    recs: &[analysis::Recommendation],
    top: usize,
    trace: Option<String>,
    report: Option<String>,
    meta: &input::RunMeta,
) -> String {
    let mut s = String::new();
    s.push_str("# nf-audit report\n\n");
    if let Some(line) = meta.summary_line() {
        s.push_str(&format!("- run: {line}\n"));
    }
    if let Some(t) = trace {
        s.push_str(&format!("- trace: `{t}`\n"));
    }
    if let Some(p) = report {
        s.push_str(&format!("- report: `{p}`\n"));
    }
    let names = display_names(run.processes.iter().map(|p| p.process.as_str()));
    s.push_str(&format!(
        "- rates: `{}` (${:.4}/CPU-h, ${:.4}/GiB-h)\n",
        r.name, r.cpu_hour, r.gib_hour
    ));
    s.push_str(&format!(
        "- tasks: {} across {} processes\n\n",
        run.tasks,
        run.processes.len()
    ));

    if run.tasks_without_requests > 0 {
        s.push_str(&format!("> {} of {} tasks lack a requested cpus and/or memory value (a default trace carries neither; a process without a `memory` directive has none). The missing part is priced on *used* resources, so it is a floor, not an allocation cost. With a trace alone, pass the execution report with `--report`.\n\n", run.tasks_without_requests, run.tasks));
    }
    if run.tasks_without_metrics > 0 {
        let all = run.tasks_without_metrics == run.tasks;
        s.push_str(&format!(
            "> {} of {} tasks carry no usage metrics (`%cpu` / `peak_rss` are `-`). {}efficiency, waste and right-sizing {} Nextflow collects these through `ps` inside the task container; a container without procps, and some Fusion/Wave combinations, leave them empty.\n\n",
            run.tasks_without_metrics, run.tasks,
            if run.tasks_without_requests == 0 { "Cost is still exact (it follows the request), but " } else { "Where a task has no request either, it adds nothing to the cost floor; " },
            if all { "cannot be computed for this run." } else { "are computed over the metered tasks only." }
        ));
    }

    let requested = |h: f64| {
        if run.tasks_without_requests == run.tasks {
            "n/a".to_string()
        } else {
            format!("{h:.1}")
        }
    };
    s.push_str("## Totals\n\n");
    s.push_str("| metric | value |\n|---|---|\n");
    let basis = if run.tasks_without_requests == 0 {
        "requested × run time"
    } else if run.tasks_without_requests == run.tasks {
        "used × run time, a floor"
    } else {
        "requested × run time; used where no request is known"
    };
    s.push_str(&format!("| cost ({basis}) | {} |\n", money(run.total_cost)));
    if run.cached_tasks > 0 {
        s.push_str(&format!(
            "| of which cached tasks (`-resume`, priced at the original run's times) | {} ({} tasks) |\n",
            money(run.cached_cost),
            run.cached_tasks
        ));
    }
    let metered_cost = run.cpu_h_req_metered * r.cpu_hour + run.gib_h_req_metered * r.gib_hour;
    if metered_cost > 0.0 {
        s.push_str(&format!(
            "| of which allocated but unused | {} ({:.0}% of the {} metered) |\n",
            money(run.total_waste),
            run.total_waste / metered_cost * 100.0,
            money(metered_cost)
        ));
    } else {
        s.push_str(
            "| of which allocated but unused | n/a (no task has both requests and usage metrics) |\n",
        );
    }
    s.push_str(&format!(
        "| of which spent on failed attempts | {} |\n",
        money(run.failed_cost)
    ));
    s.push_str(&format!(
        "| task run time (sum) | {:.1} h |\n",
        run.realtime_h
    ));
    s.push_str(&format!(
        "| CPU-hours requested / used | {} / {} ({}) |\n",
        requested(run.cpu_h_requested),
        if run.cpu_h_req_metered > 0.0 {
            format!("{:.1}", run.cpu_h_used)
        } else {
            "n/a".into()
        },
        pct(if run.cpu_h_req_metered > 0.0 {
            Some(run.cpu_h_used / run.cpu_h_req_metered)
        } else {
            None
        })
    ));
    s.push_str(&format!(
        "| GiB-hours requested / used | {} / {} ({}) |\n\n",
        requested(run.gib_h_requested),
        if run.gib_h_req_metered > 0.0 {
            format!("{:.1}", run.gib_h_used)
        } else {
            "n/a".into()
        },
        pct(if run.gib_h_req_metered > 0.0 {
            Some(run.gib_h_used / run.gib_h_req_metered)
        } else {
            None
        })
    ));

    s.push_str(&format!("## Cost by process (top {top})\n\n"));
    s.push_str("| process | tasks | run time | cost | share | cpu eff | mem eff | waste | retries | failed |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for p in run.processes.iter().take(top) {
        let short = names
            .get(p.process.as_str())
            .map(String::as_str)
            .unwrap_or(&p.process);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {:.0}% | {} | {} | {} | {} | {} |\n",
            short,
            p.tasks,
            fmt_time_h(p.realtime_h),
            money(p.cost),
            if run.total_cost > 0.0 {
                p.cost / run.total_cost * 100.0
            } else {
                0.0
            },
            pct(p.cpu_efficiency()),
            pct(p.mem_efficiency()),
            p.waste(r).map(money).unwrap_or_else(|| "n/a".into()),
            if p.attempts_known {
                p.retried_tasks.to_string()
            } else {
                "n/a".into()
            },
            p.failed_tasks
        ));
    }
    s.push('\n');

    if run.tags.iter().any(|g| g.tag.is_some()) {
        let share = |c: f64| {
            if run.total_cost > 0.0 {
                c / run.total_cost * 100.0
            } else {
                0.0
            }
        };
        let tagged: f64 = run
            .tags
            .iter()
            .filter(|g| g.tag.is_some())
            .map(|g| g.cost)
            .sum();
        s.push_str(&format!("## Cost by task tag (top {top})\n\n"));
        s.push_str(&format!(
            "The tag is the text in parentheses after the process name. nf-core pipelines usually tag with the sample ID, which makes this a per-sample cost; other tags (lanes, intervals, reference files) show up as their own rows. Tagged tasks carry {:.1}% of the cost.\n\n",
            share(tagged)
        ));
        s.push_str("| tag | tasks | processes | run time | cost | share | waste |\n|---|---:|---:|---:|---:|---:|---:|\n");
        for g in run.tags.iter().take(top) {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.0}% | {} |\n",
                g.tag.as_deref().unwrap_or("(untagged)").replace('|', "\\|"),
                g.tasks,
                g.processes,
                fmt_time_h(g.realtime_h),
                money(g.cost),
                share(g.cost),
                g.waste(r).map(money).unwrap_or_else(|| "n/a".into())
            ));
        }
        s.push('\n');
    }

    let not_sized: Vec<&str> = analysis::not_sized(run)
        .into_iter()
        .map(|p| names.get(p).map(String::as_str).unwrap_or(p))
        .collect();
    let not_sized_note = if not_sized.is_empty() {
        String::new()
    } else {
        format!(
            "\nNot sized, because fewer than half of their tasks completed with both `%cpu` and `peak_rss`: {}.\n",
            not_sized.join(", ")
        )
    };
    // A run with no metrics at all gets the one-line explanation below instead of a list.
    let not_sized_note = if run.tasks_without_metrics == run.tasks {
        String::new()
    } else {
        not_sized_note
    };
    if recs.is_empty() && !not_sized_note.is_empty() {
        s.push_str("## Right-sizing\n");
        s.push_str(&not_sized_note);
    } else if recs.is_empty() && run.tasks_without_metrics == run.tasks {
        s.push_str("## Right-sizing\n\nSkipped: no task in this run has usage metrics, so there is no observed peak to size against. The requested-vs-used comparison needs a run whose trace carries `%cpu` and `peak_rss`.\n");
    } else if recs.is_empty() && run.tasks_without_requests == run.tasks {
        s.push_str("## Right-sizing\n\nSkipped: no task has a known request, so there is nothing to shrink. Pass the run's execution report with `--report`.\n");
    }
    if !recs.is_empty() {
        let total_saving: f64 = recs.iter().map(|x| x.saving).sum();
        s.push_str(&format!(
            "## Right-sizing (margin applied to observed peaks): est. saving {} ({:.0}%)\n\n",
            money(total_saving),
            if run.total_cost > 0.0 {
                total_saving / run.total_cost * 100.0
            } else {
                0.0
            }
        ));
        s.push_str("| process | cpus now → new | memory now → new | time now → new | est. saving |\n|---|---:|---:|---:|---:|\n");
        for x in recs.iter().filter(|x| !x.needs_escalation).take(top) {
            let short = names
                .get(x.process.as_str())
                .map(String::as_str)
                .unwrap_or(&x.process);
            let time = if x.time_now_h > 0.0 {
                format!(
                    "{} → {}",
                    fmt_time_h(x.time_now_h),
                    fmt_time_h(x.time_new_h)
                )
            } else {
                "n/a".into()
            };
            s.push_str(&format!(
                "| {} | {:.0} → {} | {} → {} | {} | {} |\n",
                short,
                x.cpus_now,
                x.cpus_new,
                analysis::fmt_gib(x.mem_now_gib),
                analysis::fmt_gib(x.mem_new_gib),
                time,
                money(x.saving)
            ));
        }
        let kept: Vec<&str> = recs
            .iter()
            .filter(|x| x.needs_escalation)
            .map(|x| {
                names
                    .get(x.process.as_str())
                    .map(String::as_str)
                    .unwrap_or(&x.process)
            })
            .collect();
        if !kept.is_empty() {
            s.push_str(&format!(
                "\nLeft as they are, and not in the config fragment, because some of their tasks used more memory or time than the first attempt's request, were retried at a larger one, or were killed (exit 130–145 or 104, which nf-core treats as out of resources, or stopped at the time limit with no exit code); shrinking them could make those tasks fail: {}.\n",
                kept.join(", ")
            ));
        }
        s.push_str(&not_sized_note);
        s.push_str("\nSavings assume the same run time at the smaller allocation, which holds for memory and for CPU-bound processes that were not using the extra cores. Validate on one real run before rolling out; a process at 100% CPU efficiency will slow down if you cut its cores.\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace-only input: the cost is a floor on used resources, and nothing about requests or
    /// waste is shown as a number.
    #[test]
    fn trace_only_totals_say_unknown() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let tasks = input::read_trace(&root.join("execution_trace_synthetic.txt")).unwrap();
        let r = Rates::SEQERA_COMPUTE;
        let run = analyse(&tasks, &r);
        let md = render_markdown(
            &run,
            &r,
            &recommend(&run, &r, 1.25),
            25,
            None,
            None,
            &Default::default(),
        );
        assert!(md.contains("| cost (used × run time, a floor) |"), "{md}");
        assert!(md.contains("| of which allocated but unused | n/a"));
        assert!(md.contains("| CPU-hours requested / used | n/a /"));
        assert!(!md.contains("$0.00 (0%"));
    }

    #[test]
    fn inspect_json_serializes() {
        let info = InspectJson {
            file: "test.html".to_string(),
            kind: "report",
            meta: Some(input::RunMeta::default()),
            tasks: 10,
            tasks_with_requests: 10,
            tasks_with_metrics: 8,
            columns: vec![],
        };
        let s = serde_json::to_string(&info).unwrap();
        assert!(s.contains("\"tasks\":10"));
        assert!(s.contains("\"kind\":\"report\""));
    }
}
