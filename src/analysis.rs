//! Per-process aggregation, cost model and right-sizing recommendations.

use crate::model::Task;
use std::collections::{BTreeMap, BTreeSet};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Price per resource-hour. Two numbers are enough to price any executor that charges for
/// allocated CPU and memory; instance-level pricing is folded into them.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Rates {
    pub name: &'static str,
    pub cpu_hour: f64,
    pub gib_hour: f64,
}

impl Rates {
    /// Seqera Compute list price (Sept 2026): $0.10 per CPU-hour, $0.025 per GiB-hour.
    pub const SEQERA_COMPUTE: Rates = Rates {
        name: "seqera-compute",
        cpu_hour: 0.10,
        gib_hour: 0.025,
    };
    /// AWS m5 family on-demand, us-east-1, split into per-vCPU and per-GiB components
    /// (m5.large: $0.096/h for 2 vCPU + 8 GiB). Approximation; edit for your region.
    pub const AWS_M5_ONDEMAND: Rates = Rates {
        name: "aws-m5-ondemand",
        cpu_hour: 0.032,
        gib_hour: 0.004,
    };
    /// Same, at a typical 65% spot discount.
    pub const AWS_M5_SPOT: Rates = Rates {
        name: "aws-m5-spot",
        cpu_hour: 0.0112,
        gib_hour: 0.0014,
    };

    pub fn preset(name: &str) -> Option<Rates> {
        match name {
            "seqera-compute" => Some(Self::SEQERA_COMPUTE),
            "aws-m5-ondemand" => Some(Self::AWS_M5_ONDEMAND),
            "aws-m5-spot" => Some(Self::AWS_M5_SPOT),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessStats {
    pub process: String,
    pub tasks: usize,
    pub failed_tasks: usize,
    pub retried_tasks: usize,
    /// Whether any task recorded its `attempt` (the default trace columns do not), so that
    /// `retried_tasks` can be told apart from "unknown".
    pub attempts_known: bool,
    pub realtime_h: f64,
    pub cpu_h_requested: f64,
    pub cpu_h_used: f64,
    pub gib_h_requested: f64,
    pub gib_h_used: f64,
    /// Tasks whose trace carried `%cpu` / `peak_rss`. Efficiency and waste are computed over
    /// these only; a task without metrics is priced but says nothing about utilisation.
    pub tasks_with_metrics: usize,
    /// Tasks that recorded `%cpu`, and tasks that recorded `peak_rss`. Each metric is used only
    /// where it was recorded: a task with `%cpu` but no `peak_rss` says nothing about memory.
    pub tasks_with_cpu: usize,
    pub tasks_with_rss: usize,
    /// Requested CPU-hours over tasks with requests and `%cpu`, and GiB-hours over tasks with
    /// requests and `peak_rss`, so that `cpu_h_used / cpu_h_req_metered` compares like with like.
    pub cpu_h_req_metered: f64,
    pub gib_h_req_metered: f64,
    pub max_cpus_requested: f64,
    pub max_cpu_used_cores: f64,
    pub max_mem_requested_gib: f64,
    pub max_rss_gib: f64,
    /// Largest request among first attempts: the process's own setting, before any retry
    /// escalation. Right-sizing compares against this. 0 when no first attempt was seen.
    pub base_cpus_requested: f64,
    pub base_mem_requested_gib: f64,
    pub base_time_limit_h: f64,
    pub max_realtime_h: f64,
    pub max_time_limit_h: f64,
    pub cost: f64,
    pub failed_cost: f64,
    pub has_requests: bool,
}

impl ProcessStats {
    pub fn cpu_efficiency(&self) -> Option<f64> {
        if self.cpu_h_req_metered > 0.0 {
            Some(self.cpu_h_used / self.cpu_h_req_metered)
        } else {
            None
        }
    }
    pub fn mem_efficiency(&self) -> Option<f64> {
        if self.gib_h_req_metered > 0.0 {
            Some(self.gib_h_used / self.gib_h_req_metered)
        } else {
            None
        }
    }
    /// Dollars spent on allocation that was never used, under the given rates. Only tasks with
    /// both usage metrics and requests contribute; for the rest the waste is unknown, not zero.
    pub fn waste(&self, r: &Rates) -> Option<f64> {
        if self.cpu_h_req_metered <= 0.0 && self.gib_h_req_metered <= 0.0 {
            return None;
        }
        let cpu_waste = (self.cpu_h_req_metered - self.cpu_h_used).max(0.0) * r.cpu_hour;
        let mem_waste = (self.gib_h_req_metered - self.gib_h_used).max(0.0) * r.gib_hour;
        Some(cpu_waste + mem_waste)
    }
}

/// Cost grouped by task tag (see [`Task::tag_from_name`]). In nf-core pipelines the tag is
/// usually the sample ID, which makes this a per-sample cost, but the tag is whatever the
/// pipeline author chose, so the table says "tag", not "sample".
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagStats {
    /// `None` collects every task without a tag.
    pub tag: Option<String>,
    pub tasks: usize,
    /// Distinct processes that ran under this tag.
    pub processes: usize,
    pub realtime_h: f64,
    pub cost: f64,
    pub failed_cost: f64,
    pub tasks_with_metrics: usize,
    pub cpu_h_req_metered: f64,
    pub cpu_h_used: f64,
    pub gib_h_req_metered: f64,
    pub gib_h_used: f64,
}

impl TagStats {
    /// Same definition as [`ProcessStats::waste`], over this tag's tasks.
    pub fn waste(&self, r: &Rates) -> Option<f64> {
        if self.cpu_h_req_metered <= 0.0 && self.gib_h_req_metered <= 0.0 {
            return None;
        }
        let cpu_waste = (self.cpu_h_req_metered - self.cpu_h_used).max(0.0) * r.cpu_hour;
        let mem_waste = (self.gib_h_req_metered - self.gib_h_used).max(0.0) * r.gib_hour;
        Some(cpu_waste + mem_waste)
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunStats {
    pub tasks: usize,
    pub processes: Vec<ProcessStats>,
    /// Cost by task tag, most expensive first.
    pub tags: Vec<TagStats>,
    pub total_cost: f64,
    pub total_waste: f64,
    pub failed_cost: f64,
    pub realtime_h: f64,
    pub cpu_h_requested: f64,
    pub cpu_h_used: f64,
    pub gib_h_requested: f64,
    pub gib_h_used: f64,
    pub tasks_without_requests: usize,
    /// `CACHED` tasks: reused from an earlier run by `-resume`. They are priced at the run time
    /// recorded for them, which is what the original run spent, not what this run did.
    pub cached_tasks: usize,
    pub cached_cost: f64,
    /// Tasks with no `%cpu` / `peak_rss` in the trace. Common on AWS Batch runs where the
    /// container lacks `ps`, and on some Fusion/Wave combinations; Nextflow then writes `-`.
    pub tasks_without_metrics: usize,
    /// Requested resource-hours over the metered subset only (denominator for the totals' efficiency).
    pub cpu_h_req_metered: f64,
    pub gib_h_req_metered: f64,
}

pub fn analyse(tasks: &[Task], rates: &Rates) -> RunStats {
    let mut by_proc: BTreeMap<String, ProcessStats> = BTreeMap::new();
    let mut by_tag: BTreeMap<Option<String>, (TagStats, BTreeSet<String>)> = BTreeMap::new();
    let mut run = RunStats {
        tasks: tasks.len(),
        ..Default::default()
    };

    for t in tasks {
        let hours = t.realtime_s.unwrap_or(0.0) / 3600.0;
        let cpus_req = t.cpus;
        let mem_req_gib = t.memory.map(|b| b / GIB);
        let has_metrics = t.pct_cpu.is_some() || t.peak_rss.is_some();
        // Used and requested are compared only over tasks that carry both, per metric.
        let has_req = cpus_req.is_some() && mem_req_gib.is_some();
        let cpu_paired = has_req && t.pct_cpu.is_some();
        let mem_paired = has_req && t.peak_rss.is_some();
        let cpu_used_cores = t.pct_cpu.map(|p| p / 100.0).unwrap_or(0.0);
        let rss_gib = t.peak_rss.map(|b| b / GIB).unwrap_or(0.0);

        let p = by_proc
            .entry(t.process.clone())
            .or_insert_with(|| ProcessStats {
                process: t.process.clone(),
                ..Default::default()
            });
        p.tasks += 1;
        p.realtime_h += hours;
        p.max_realtime_h = p.max_realtime_h.max(hours);
        if let Some(tl) = t.time_s {
            p.max_time_limit_h = p.max_time_limit_h.max(tl / 3600.0);
        }
        if t.attempt.is_some() {
            p.attempts_known = true;
        }
        if t.attempt.unwrap_or(1) > 1 {
            p.retried_tasks += 1;
        }
        let failed = !t.succeeded();
        if failed {
            p.failed_tasks += 1;
        }

        // Used resources: what the task actually consumed, integrated over its runtime.
        // A task without metrics contributes nothing here and is not counted as 0% used.
        let cpu_h_used = cpu_used_cores * hours;
        let gib_h_used = rss_gib * hours;
        if cpu_paired {
            p.cpu_h_used += cpu_h_used;
        }
        if mem_paired {
            p.gib_h_used += gib_h_used;
        }
        if t.pct_cpu.is_some() {
            p.tasks_with_cpu += 1;
            p.max_cpu_used_cores = p.max_cpu_used_cores.max(cpu_used_cores);
        }
        if t.peak_rss.is_some() {
            p.tasks_with_rss += 1;
            p.max_rss_gib = p.max_rss_gib.max(rss_gib);
        }
        if has_metrics {
            p.tasks_with_metrics += 1;
        } else {
            run.tasks_without_metrics += 1;
        }

        // Requested resources: what the scheduler had to reserve, hence what is priced.
        let (cost, cpu_h_req_metered, gib_h_req_metered) = match (cpus_req, mem_req_gib) {
            (Some(c), Some(m)) => {
                p.has_requests = true;
                p.cpu_h_requested += c * hours;
                p.gib_h_requested += m * hours;
                p.max_cpus_requested = p.max_cpus_requested.max(c);
                p.max_mem_requested_gib = p.max_mem_requested_gib.max(m);
                if t.attempt.unwrap_or(1) == 1 {
                    p.base_cpus_requested = p.base_cpus_requested.max(c);
                    p.base_mem_requested_gib = p.base_mem_requested_gib.max(m);
                    if let Some(tl) = t.time_s {
                        p.base_time_limit_h = p.base_time_limit_h.max(tl / 3600.0);
                    }
                }
                let metered = (
                    if cpu_paired { c * hours } else { 0.0 },
                    if mem_paired { m * hours } else { 0.0 },
                );
                (
                    c * hours * rates.cpu_hour + m * hours * rates.gib_hour,
                    metered.0,
                    metered.1,
                )
            }
            _ => {
                // No request data (plain TSV trace): fall back to pricing what was used, and
                // flag it so the report says the number is a floor, not an audit.
                run.tasks_without_requests += 1;
                (
                    cpu_h_used * rates.cpu_hour + gib_h_used * rates.gib_hour,
                    0.0,
                    0.0,
                )
            }
        };
        p.cost += cost;
        if t.status == "CACHED" {
            run.cached_tasks += 1;
            run.cached_cost += cost;
        }
        p.cpu_h_req_metered += cpu_h_req_metered;
        p.gib_h_req_metered += gib_h_req_metered;
        if failed {
            p.failed_cost += cost;
        }

        let tag = t.tag.clone();
        let (g, procs) = by_tag.entry(tag.clone()).or_insert_with(|| {
            (
                TagStats {
                    tag,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
        });
        procs.insert(t.process.clone());
        g.tasks += 1;
        g.realtime_h += hours;
        g.cost += cost;
        if failed {
            g.failed_cost += cost;
        }
        if has_metrics {
            g.tasks_with_metrics += 1;
        }
        if cpu_paired {
            g.cpu_h_used += cpu_h_used;
        }
        if mem_paired {
            g.gib_h_used += gib_h_used;
        }
        g.cpu_h_req_metered += cpu_h_req_metered;
        g.gib_h_req_metered += gib_h_req_metered;
    }

    let mut tags: Vec<TagStats> = by_tag
        .into_values()
        .map(|(mut g, procs)| {
            g.processes = procs.len();
            g
        })
        .collect();
    tags.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    run.tags = tags;

    let mut procs: Vec<ProcessStats> = by_proc.into_values().collect();
    procs.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for p in &procs {
        run.total_cost += p.cost;
        run.total_waste += p.waste(rates).unwrap_or(0.0);
        run.failed_cost += p.failed_cost;
        run.realtime_h += p.realtime_h;
        run.cpu_h_requested += p.cpu_h_requested;
        run.cpu_h_used += p.cpu_h_used;
        run.gib_h_requested += p.gib_h_requested;
        run.gib_h_used += p.gib_h_used;
        run.cpu_h_req_metered += p.cpu_h_req_metered;
        run.gib_h_req_metered += p.gib_h_req_metered;
    }
    run.processes = procs;
    run
}

/// A right-sizing recommendation for one process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Recommendation {
    pub process: String,
    pub cpus_now: f64,
    pub cpus_new: u32,
    pub mem_now_gib: f64,
    pub mem_new_gib: f64,
    pub time_now_h: f64,
    pub time_new_h: f64,
    pub saving: f64,
    /// Set when some task needed more than the first attempt's request (its peak memory or its
    /// run time exceeds it). The process relies on the pipeline's retry
    /// escalation, so no change is recommended and nothing is written to the config fragment.
    pub needs_escalation: bool,
}

/// Recommend per-process requests: observed peak times a safety margin, rounded to sane units,
/// never below 1 CPU / 1 GiB unless the current request is lower, and never *above* the current
/// request (this tool shrinks over-allocation; it does not diagnose OOM kills, which show up as
/// failed tasks instead). "Current" is the first-attempt request, before retry escalation. A
/// resource with no observation (`%cpu` or `peak_rss` never recorded) keeps its current request.
pub fn recommend(run: &RunStats, rates: &Rates, margin: f64) -> Vec<Recommendation> {
    let mut out = Vec::new();
    for p in &run.processes {
        // No requests: nothing to shrink. No metrics: no observed peak to shrink to, and a
        // recommendation of "1 CPU / 1 GiB" from a zero would be wrong and dangerous.
        if !p.has_requests || p.tasks == 0 || p.tasks_with_metrics == 0 {
            continue;
        }
        let base = |b: f64, max: f64| if b > 0.0 { b } else { max };
        let cpus_now = base(p.base_cpus_requested, p.max_cpus_requested);
        let mem_now = base(p.base_mem_requested_gib, p.max_mem_requested_gib);
        let cpus_new = if p.tasks_with_cpu > 0 {
            ((p.max_cpu_used_cores * margin).ceil() as u32)
                .max(1)
                .min(cpus_now.ceil() as u32)
        } else {
            cpus_now.ceil() as u32
        };
        let mem_new = if p.tasks_with_rss > 0 {
            round_mem_gib(p.max_rss_gib * margin).max(1.0).min(mem_now)
        } else {
            mem_now
        };
        let time_now = base(p.base_time_limit_h, p.max_time_limit_h);
        let time_new = if time_now > 0.0 {
            round_time_h(p.max_realtime_h * margin.max(1.5)).min(time_now)
        } else {
            0.0
        };
        // A fixed value replaces the pipeline's retry escalation, so a process whose tasks used
        // more than the first attempt's request (peak memory or run time) is left alone: writing the
        // first-attempt value back would make those tasks fail on every attempt.
        let needs_escalation = (p.tasks_with_rss > 0 && p.max_rss_gib > mem_now)
            || (time_now > 0.0 && p.max_realtime_h > time_now);
        if needs_escalation {
            out.push(Recommendation {
                process: p.process.clone(),
                cpus_now,
                cpus_new: cpus_now.ceil() as u32,
                mem_now_gib: mem_now,
                mem_new_gib: mem_now,
                time_now_h: time_now,
                time_new_h: time_now,
                saving: 0.0,
                needs_escalation,
            });
            continue;
        }
        // Saving is estimated by re-pricing the same runtime at the new allocation.
        let new_cost = (cpus_new as f64) * p.realtime_h * rates.cpu_hour
            + mem_new * p.realtime_h * rates.gib_hour;
        let saving = (p.cost - new_cost).max(0.0);
        out.push(Recommendation {
            process: p.process.clone(),
            cpus_now,
            cpus_new,
            mem_now_gib: mem_now,
            mem_new_gib: mem_new,
            time_now_h: time_now,
            time_new_h: time_new,
            saving,
            needs_escalation,
        });
    }
    out.sort_by(|a, b| {
        b.saving
            .partial_cmp(&a.saving)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn round_mem_gib(g: f64) -> f64 {
    if g <= 1.0 {
        1.0
    } else if g <= 8.0 {
        g.ceil()
    } else if g <= 64.0 {
        (g / 2.0).ceil() * 2.0
    } else {
        (g / 8.0).ceil() * 8.0
    }
}

fn round_time_h(h: f64) -> f64 {
    if h <= 0.25 {
        0.25
    } else if h <= 1.0 {
        (h * 4.0).ceil() / 4.0
    } else {
        h.ceil()
    }
}

/// The `nextflow.config` fragment for these recommendations: one `withName` block per process
/// with a positive saving. Values are written exactly (rounded down to whole MB or seconds where
/// they are not whole GB or hours), so the fragment never asks for more than the current request.
pub fn render_config(recs: &[Recommendation]) -> String {
    let mut s = String::from(
        "// Generated by nf-audit. Review before use; apply with `-c nf-audit.config`.\n\
         // Fixed values replace any retry escalation the pipeline sets for these processes\n\
         // (e.g. `memory = { 8.GB * task.attempt }`), so a task that runs out retries at the same size.\n\
         process {\n",
    );
    for x in recs {
        if x.saving <= 0.0 {
            continue;
        }
        s.push_str(&format!(
            "    withName: '{}' {{\n        cpus   = {}\n        memory = '{}'\n",
            x.process,
            x.cpus_new,
            nf_memory(x.mem_new_gib)
        ));
        if x.time_new_h > 0.0 {
            s.push_str(&format!(
                "        time   = '{}'\n",
                nf_duration(x.time_new_h)
            ));
        }
        s.push_str("    }\n");
    }
    s.push_str("}\n");
    s
}

/// Nextflow `MemoryUnit` literal for `gib`, rounded down: `12.GB`, else `3584.MB`, else `512.KB`.
fn nf_memory(gib: f64) -> String {
    let mb = (gib * 1024.0 + 1e-6).floor() as u64;
    if mb.is_multiple_of(1024) && mb > 0 {
        format!("{}.GB", mb / 1024)
    } else if mb > 0 {
        format!("{mb}.MB")
    } else {
        format!(
            "{}.KB",
            ((gib * 1024.0 * 1024.0 + 1e-6).floor() as u64).max(1)
        )
    }
}

/// Nextflow `Duration` literal for `h` hours, rounded down: `16.h`, else `90.m`, else `75.s`.
fn nf_duration(h: f64) -> String {
    let secs = ((h * 3600.0 + 1e-6).floor() as u64).max(1);
    if secs.is_multiple_of(3600) {
        format!("{}.h", secs / 3600)
    } else if secs.is_multiple_of(60) {
        format!("{}.m", secs / 60)
    } else {
        format!("{secs}.s")
    }
}

/// `12 GB`, `3.5 GB`, `512 MB`, `243 KB`: the largest unit that keeps the value at or above 1,
/// with at most two decimals. (Nextflow's `GB` is GiB.)
pub fn fmt_gib(g: f64) -> String {
    let (v, unit) = if g >= 1.0 {
        (g, "GB")
    } else if g * 1024.0 >= 1.0 {
        (g * 1024.0, "MB")
    } else {
        (g * 1024.0 * 1024.0, "KB")
    };
    let t = format!("{v:.2}");
    format!("{} {unit}", t.trim_end_matches('0').trim_end_matches('.'))
}

pub fn fmt_time_h(h: f64) -> String {
    if h < 1.0 {
        format!("{}m", (h * 60.0).round() as u32)
    } else if (h - h.round()).abs() < 1e-9 {
        format!("{}h", h as u32)
    } else {
        format!("{:.1}h", h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{merge, read_report_with_meta, read_trace};
    use std::path::Path;

    fn fixtures() -> Vec<Task> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let trace = read_trace(&root.join("execution_trace_synthetic.txt")).unwrap();
        let (report, _) =
            read_report_with_meta(&root.join("execution_report_synthetic.html")).unwrap();
        merge(trace, report)
    }

    fn rec(mem_now: f64, mem_new: f64, time_now: f64, time_new: f64) -> Recommendation {
        Recommendation {
            process: "P:X".into(),
            cpus_now: 4.0,
            cpus_new: 1,
            mem_now_gib: mem_now,
            mem_new_gib: mem_new,
            time_now_h: time_now,
            time_new_h: time_new,
            saving: 1.0,
            needs_escalation: false,
        }
    }

    /// The fragment must never round a request up past the current one, or down to zero.
    #[test]
    fn config_values_are_exact_and_never_above_the_request() {
        let cfg = |r: Recommendation| render_config(&[r]);
        assert!(cfg(rec(0.5, 0.5, 1.0, 1.0)).contains("memory = '512.MB'"));
        assert!(cfg(rec(3.5, 3.5, 1.0, 1.0)).contains("memory = '3584.MB'"));
        assert!(cfg(rec(2.5, 2.5, 1.0, 1.0)).contains("memory = '2560.MB'"));
        assert!(cfg(rec(72.0, 12.0, 16.0, 16.0)).contains("memory = '12.GB'"));
        assert!(cfg(rec(1.0, 1.0, 1.5, 1.5)).contains("time   = '90.m'"));
        assert!(cfg(rec(1.0, 1.0, 16.0, 15.0)).contains("time   = '15.h'"));
        assert!(cfg(rec(1.0, 1.0, 1.0, 0.25)).contains("time   = '15.m'"));
        assert!(cfg(rec(1.0, 1.0, 0.01, 0.01)).contains("time   = '36.s'"));
        let mut none = rec(1.0, 1.0, 1.0, 1.0);
        none.saving = 0.0;
        assert!(!render_config(&[none]).contains("withName"));
    }

    #[test]
    fn gib_display() {
        assert_eq!(fmt_gib(12.0), "12 GB");
        assert_eq!(fmt_gib(3.5), "3.5 GB");
        assert_eq!(fmt_gib(0.5), "512 MB");
        assert_eq!(fmt_gib(244.0 / 1024.0 / 1024.0), "244 KB");
    }

    #[test]
    fn synthetic_run_prices_and_meters_every_task() {
        let tasks = fixtures();
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert_eq!(run.tasks, 37);
        assert_eq!(run.tasks_without_requests, 0);
        assert_eq!(run.tasks_without_metrics, 0);
        assert!(run.total_cost > 0.0);
        assert!(run.total_waste > 0.0 && run.total_waste < run.total_cost);
        // Totals are the sum of the per-process rows.
        let by_proc: f64 = run.processes.iter().map(|p| p.cost).sum();
        assert!((by_proc - run.total_cost).abs() < 1e-9);
        // Sorted by cost, descending.
        assert!(run.processes.windows(2).all(|w| w[0].cost >= w[1].cost));
        // Every process is metered, so every process gets a recommendation no larger than now.
        let recs = recommend(&run, &Rates::SEQERA_COMPUTE, 1.25);
        assert_eq!(recs.len(), run.processes.len());
        for r in &recs {
            assert!(f64::from(r.cpus_new) <= r.cpus_now.ceil());
            assert!(r.mem_new_gib <= r.mem_now_gib);
            assert!(r.cpus_new >= 1 && r.mem_new_gib >= 1.0);
        }
    }

    #[test]
    fn cached_tasks_are_counted_and_priced() {
        let mut tasks = fixtures();
        let base = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert_eq!((base.cached_tasks, base.cached_cost), (0, 0.0));
        tasks[0].status = "CACHED".into();
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert_eq!(run.cached_tasks, 1);
        assert!(run.cached_cost > 0.0);
        assert!((run.total_cost - base.total_cost).abs() < 1e-9);
    }

    #[test]
    fn cost_by_tag_partitions_the_run() {
        let run = analyse(&fixtures(), &Rates::SEQERA_COMPUTE);
        // 8 sample tags plus MULTIQC, which has no tag.
        assert_eq!(run.tags.len(), 9);
        let tasks: usize = run.tags.iter().map(|g| g.tasks).sum();
        assert_eq!(tasks, run.tasks);
        let cost: f64 = run.tags.iter().map(|g| g.cost).sum();
        assert!((cost - run.total_cost).abs() < 1e-9);
        assert!(run.tags.windows(2).all(|w| w[0].cost >= w[1].cost));

        let untagged = run.tags.iter().find(|g| g.tag.is_none()).unwrap();
        assert_eq!((untagged.tasks, untagged.processes), (1, 1));
        // Retried STAR and PICARD attempts count as tasks of the same tag.
        let s = run
            .tags
            .iter()
            .find(|g| g.tag.as_deref() == Some("SRX1603392_T1"))
            .unwrap();
        assert_eq!((s.tasks, s.processes), (6, 4));
        assert!(s.waste(&Rates::SEQERA_COMPUTE).unwrap() <= s.cost);
    }

    #[test]
    fn missing_metrics_are_unknown_not_zero() {
        let mut tasks = fixtures();
        for t in &mut tasks {
            t.pct_cpu = None;
            t.peak_rss = None;
        }
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert_eq!(run.tasks_without_metrics, run.tasks);
        assert_eq!(run.total_waste, 0.0);
        assert!(run
            .processes
            .iter()
            .all(|p| p.cpu_efficiency().is_none() && p.waste(&Rates::SEQERA_COMPUTE).is_none()));
        assert!(recommend(&run, &Rates::SEQERA_COMPUTE, 1.25).is_empty());
        // Cost does not depend on metrics at all.
        let metered = analyse(&fixtures(), &Rates::SEQERA_COMPUTE);
        assert!((metered.total_cost - run.total_cost).abs() < 1e-9);
    }

    /// Trace-only input has usage but no requests: waste and efficiency are unknown, not $0 / 0%.
    #[test]
    fn trace_only_waste_is_unknown() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let trace = read_trace(&root.join("execution_trace_synthetic.txt")).unwrap();
        let run = analyse(&trace, &Rates::SEQERA_COMPUTE);
        assert_eq!(run.tasks_without_requests, run.tasks);
        assert!(run.total_cost > 0.0, "priced on used resources");
        for p in &run.processes {
            assert_eq!(p.waste(&Rates::SEQERA_COMPUTE), None, "{}", p.process);
            assert_eq!(p.cpu_efficiency(), None);
        }
        assert!(run
            .tags
            .iter()
            .all(|g| g.waste(&Rates::SEQERA_COMPUTE).is_none()));
        assert_eq!(run.total_waste, 0.0);
    }

    /// A metered task without requests must not inflate the used side of an efficiency ratio
    /// whose requested side it is absent from.
    #[test]
    fn used_counts_only_where_requests_are_known() {
        let mut tasks = fixtures();
        let before = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        let p0 = before.processes[0].process.clone();
        let eff0 = before.processes[0].cpu_efficiency().unwrap();
        let mut extra = tasks.iter().find(|t| t.process == p0).unwrap().clone();
        extra.hash = "zz/extra".into();
        extra.cpus = None;
        extra.memory = None;
        tasks.push(extra);
        let after = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        let p = after.processes.iter().find(|p| p.process == p0).unwrap();
        assert!((p.cpu_efficiency().unwrap() - eff0).abs() < 1e-12);
    }

    /// A task with `%cpu` but no `peak_rss` must not read as 0 GiB used: memory efficiency and
    /// waste stay unknown and the memory request is left as it is.
    #[test]
    fn one_metric_does_not_stand_in_for_the_other() {
        let mut tasks = fixtures();
        for t in &mut tasks {
            t.peak_rss = None;
        }
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        for p in &run.processes {
            assert_eq!(p.mem_efficiency(), None, "{}", p.process);
            assert!(p.cpu_efficiency().is_some());
        }
        for r in recommend(&run, &Rates::SEQERA_COMPUTE, 1.25) {
            assert_eq!(r.mem_new_gib, r.mem_now_gib, "{}", r.process);
        }
    }

    /// Retries escalate the request (`12 / 72` after `6 / 36`); "now" is the first-attempt value.
    #[test]
    fn current_request_is_the_first_attempt() {
        let mut tasks = fixtures();
        let p0 = tasks[0].process.clone();
        let mut retry = tasks[0].clone();
        retry.hash = "zz/retry".into();
        retry.attempt = Some(2);
        retry.cpus = retry.cpus.map(|c| c * 2.0);
        retry.memory = retry.memory.map(|m| m * 2.0);
        let base = tasks[0].cpus.unwrap();
        tasks.push(retry);
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        let rec = recommend(&run, &Rates::SEQERA_COMPUTE, 1.25);
        let r = rec.iter().find(|r| r.process == p0).unwrap();
        assert_eq!(r.cpus_now, base);
        assert!(f64::from(r.cpus_new) <= base);
    }

    /// Attempt 1 at 8 GiB / 4 h fails; attempt 2 at 16 GiB / 8 h peaks at 12 GiB. Pinning the
    /// first-attempt values would fail every time, so the process is left to its escalation.
    #[test]
    fn processes_that_needed_escalation_are_left_alone() {
        let mut tasks = fixtures();
        let p0 = tasks[0].process.clone();
        let base = tasks.iter().find(|t| t.process == p0).unwrap().clone();
        tasks.retain(|t| t.process != p0);
        let mut first = base.clone();
        first.hash = "aa/1".into();
        first.attempt = Some(1);
        first.status = "FAILED".into();
        first.memory = Some(8.0 * GIB);
        first.time_s = Some(4.0 * 3600.0);
        first.peak_rss = Some(7.9 * GIB);
        let mut second = base;
        second.hash = "aa/2".into();
        second.attempt = Some(2);
        second.status = "COMPLETED".into();
        second.memory = Some(16.0 * GIB);
        second.time_s = Some(8.0 * 3600.0);
        second.peak_rss = Some(12.0 * GIB);
        tasks.push(first);
        tasks.push(second);
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        let rec = recommend(&run, &Rates::SEQERA_COMPUTE, 1.25);
        let r = rec.iter().find(|r| r.process == p0).unwrap();
        assert!(r.needs_escalation);
        assert_eq!(
            (r.mem_now_gib, r.mem_new_gib, r.time_now_h),
            (8.0, 8.0, 4.0)
        );
        assert_eq!(r.saving, 0.0);
        assert!(!render_config(&rec).contains(&format!("'{p0}'")));
    }

    #[test]
    fn partial_metrics_compare_like_with_like() {
        let mut tasks = fixtures();
        // Strip metrics from half the tasks: efficiency must be computed over the other half only,
        // so it stays within (0, 1] instead of being dragged towards zero.
        for t in tasks.iter_mut().step_by(2) {
            t.pct_cpu = None;
            t.peak_rss = None;
        }
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert!(run.tasks_without_metrics > 0 && run.tasks_without_metrics < run.tasks);
        assert!(run.cpu_h_req_metered < run.cpu_h_requested);
        let eff = run.cpu_h_used / run.cpu_h_req_metered;
        assert!(eff > 0.0 && eff <= 1.0, "{eff}");
    }
}
