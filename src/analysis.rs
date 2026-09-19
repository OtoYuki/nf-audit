//! Per-process aggregation, cost model and right-sizing recommendations.

use crate::model::Task;
use std::collections::BTreeMap;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Price per resource-hour. Two numbers are enough to price any executor that bills on
/// allocated CPU and memory; instance-level pricing is folded into them.
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Default, Clone)]
pub struct ProcessStats {
    pub process: String,
    pub tasks: usize,
    pub failed_tasks: usize,
    pub retried_tasks: usize,
    pub realtime_h: f64,
    pub cpu_h_requested: f64,
    pub cpu_h_used: f64,
    pub gib_h_requested: f64,
    pub gib_h_used: f64,
    /// Tasks whose trace carried `%cpu` / `peak_rss`. Efficiency and waste are computed over
    /// these only; a task without metrics is billed but says nothing about utilisation.
    pub tasks_with_metrics: usize,
    /// Requested CPU-hours / GiB-hours restricted to tasks with metrics, so that
    /// `cpu_h_used / cpu_h_req_metered` compares like with like.
    pub cpu_h_req_metered: f64,
    pub gib_h_req_metered: f64,
    pub max_cpus_requested: f64,
    pub max_cpu_used_cores: f64,
    pub max_mem_requested_gib: f64,
    pub max_rss_gib: f64,
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
    /// usage metrics contribute; for the rest the waste is unknown, not zero.
    pub fn waste(&self, r: &Rates) -> Option<f64> {
        if self.tasks_with_metrics == 0 {
            return None;
        }
        let cpu_waste = (self.cpu_h_req_metered - self.cpu_h_used).max(0.0) * r.cpu_hour;
        let mem_waste = (self.gib_h_req_metered - self.gib_h_used).max(0.0) * r.gib_hour;
        Some(cpu_waste + mem_waste)
    }
}

#[derive(Debug, Default)]
pub struct RunStats {
    pub tasks: usize,
    pub processes: Vec<ProcessStats>,
    pub total_cost: f64,
    pub total_waste: f64,
    pub failed_cost: f64,
    pub realtime_h: f64,
    pub cpu_h_requested: f64,
    pub cpu_h_used: f64,
    pub gib_h_requested: f64,
    pub gib_h_used: f64,
    pub tasks_without_requests: usize,
    /// Tasks with no `%cpu` / `peak_rss` in the trace. Common on AWS Batch runs where the
    /// container lacks `ps`, and on some Fusion/Wave combinations; Nextflow then writes `-`.
    pub tasks_without_metrics: usize,
    /// Requested resource-hours over the metered subset only (denominator for the totals' efficiency).
    pub cpu_h_req_metered: f64,
    pub gib_h_req_metered: f64,
}

pub fn analyse(tasks: &[Task], rates: &Rates) -> RunStats {
    let mut by_proc: BTreeMap<String, ProcessStats> = BTreeMap::new();
    let mut run = RunStats {
        tasks: tasks.len(),
        ..Default::default()
    };

    for t in tasks {
        let hours = t.realtime_s.unwrap_or(0.0) / 3600.0;
        let cpus_req = t.cpus;
        let mem_req_gib = t.memory.map(|b| b / GIB);
        let has_metrics = t.pct_cpu.is_some() || t.peak_rss.is_some();
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
        if has_metrics {
            p.tasks_with_metrics += 1;
            p.cpu_h_used += cpu_h_used;
            p.gib_h_used += gib_h_used;
            p.max_cpu_used_cores = p.max_cpu_used_cores.max(cpu_used_cores);
            p.max_rss_gib = p.max_rss_gib.max(rss_gib);
        } else {
            run.tasks_without_metrics += 1;
        }

        // Requested resources: what the scheduler had to reserve, hence what is billed.
        match (cpus_req, mem_req_gib) {
            (Some(c), Some(m)) => {
                p.has_requests = true;
                p.cpu_h_requested += c * hours;
                p.gib_h_requested += m * hours;
                if has_metrics {
                    p.cpu_h_req_metered += c * hours;
                    p.gib_h_req_metered += m * hours;
                }
                p.max_cpus_requested = p.max_cpus_requested.max(c);
                p.max_mem_requested_gib = p.max_mem_requested_gib.max(m);
                let cost = c * hours * rates.cpu_hour + m * hours * rates.gib_hour;
                p.cost += cost;
                if failed {
                    p.failed_cost += cost;
                }
            }
            _ => {
                // No request data (plain TSV trace): fall back to pricing what was used, and
                // flag it so the report says the number is a floor, not an audit.
                run.tasks_without_requests += 1;
                let cost = cpu_h_used * rates.cpu_hour + gib_h_used * rates.gib_hour;
                p.cost += cost;
                if failed {
                    p.failed_cost += cost;
                }
            }
        }
    }

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
#[derive(Debug)]
pub struct Recommendation {
    pub process: String,
    pub cpus_now: f64,
    pub cpus_new: u32,
    pub mem_now_gib: f64,
    pub mem_new_gib: f64,
    pub time_now_h: f64,
    pub time_new_h: f64,
    pub saving: f64,
}

/// Recommend per-process requests: observed peak times a safety margin, rounded to sane units,
/// never below 1 CPU / 1 GiB, and never *above* the current request (this tool shrinks
/// over-allocation; it does not diagnose OOM kills, which show up as failed tasks instead).
pub fn recommend(run: &RunStats, rates: &Rates, margin: f64) -> Vec<Recommendation> {
    let mut out = Vec::new();
    for p in &run.processes {
        // No requests: nothing to shrink. No metrics: no observed peak to shrink to, and a
        // recommendation of "1 CPU / 1 GiB" from a zero would be wrong and dangerous.
        if !p.has_requests || p.tasks == 0 || p.tasks_with_metrics == 0 {
            continue;
        }
        let cpus_new = ((p.max_cpu_used_cores * margin).ceil() as u32)
            .max(1)
            .min(p.max_cpus_requested.ceil() as u32);
        let mem_new = round_mem_gib(p.max_rss_gib * margin)
            .max(1.0)
            .min(p.max_mem_requested_gib);
        let time_new = if p.max_time_limit_h > 0.0 {
            round_time_h(p.max_realtime_h * margin.max(1.5)).min(p.max_time_limit_h)
        } else {
            0.0
        };
        // Saving is estimated by re-pricing the same runtime at the new allocation.
        let new_cost = (cpus_new as f64) * p.realtime_h * rates.cpu_hour
            + mem_new * p.realtime_h * rates.gib_hour;
        let saving = (p.cost - new_cost).max(0.0);
        out.push(Recommendation {
            process: p.process.clone(),
            cpus_now: p.max_cpus_requested,
            cpus_new,
            mem_now_gib: p.max_mem_requested_gib,
            mem_new_gib: mem_new,
            time_now_h: p.max_time_limit_h,
            time_new_h: time_new,
            saving,
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
