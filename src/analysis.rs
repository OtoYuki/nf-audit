//! Per-process aggregation, cost model and right-sizing recommendations.

use crate::model::Task;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    /// Every built-in preset, in the order `--rates` documents them.
    pub const PRESETS: [Rates; 3] = [
        Self::SEQERA_COMPUTE,
        Self::AWS_M5_ONDEMAND,
        Self::AWS_M5_SPOT,
    ];

    /// The cost of a run already analysed, at these rates instead.
    pub fn price(&self, run: &RunStats) -> f64 {
        run.priced_cpu_h * self.cpu_hour + run.priced_gib_h * self.gib_hour
    }

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
    /// Largest request among retries (attempt > 1); above the base when the pipeline escalates.
    pub retry_cpus_requested: f64,
    pub retry_mem_requested_gib: f64,
    pub retry_time_limit_h: f64,
    /// Failed tasks whose exit code is one nf-core retries as a resource failure (130–145, a
    /// signal such as 137 = SIGKILL, typically out of memory; or 104). They often record no
    /// metrics, so no observed peak shows what they would have needed.
    pub killed_tasks: usize,
    /// Completed (or cached) tasks that recorded both `%cpu` and `peak_rss`.
    pub completed_with_metrics: usize,
    pub max_realtime_h: f64,
    pub max_time_limit_h: f64,
    pub cost: f64,
    pub failed_cost: f64,
    pub has_requests: bool,
    /// Tasks by request group, smallest memory first. A pipeline can set a request per task (from
    /// the input size, say), so one process can hold tasks that asked for 16 GB and 96 GB.
    #[serde(default)]
    pub groups: Vec<RequestGroup>,
    /// Tasks that belong to no group: no first-attempt request of their own, and a name that does
    /// not single out one first attempt (untagged tasks share their name).
    #[serde(default)]
    pub ungrouped: GroupUsage,
}

/// The tasks whose first attempt made the same request, with their retries.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestGroup {
    pub cpus: f64,
    pub memory_gib: f64,
    /// 0 when the tasks carry no time limit.
    pub time_limit_h: f64,
    pub usage: GroupUsage,
}

/// What a set of tasks used, and how far their retries escalated.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupUsage {
    pub tasks: usize,
    pub retried_tasks: usize,
    pub killed_tasks: usize,
    pub tasks_with_rss: usize,
    pub max_rss_gib: f64,
    pub max_realtime_h: f64,
    pub realtime_h: f64,
    /// Largest request among retries (attempt > 1).
    pub retry_cpus_requested: f64,
    pub retry_mem_requested_gib: f64,
    pub retry_time_limit_h: f64,
}

impl GroupUsage {
    /// Whether these tasks used, or retried at, more than the given request, or were killed: the
    /// same test [`recommend`] applies to a whole process.
    fn needs_more_than(&self, cpus: f64, mem_gib: f64, time_h: f64) -> bool {
        (self.tasks_with_rss > 0 && self.max_rss_gib > mem_gib)
            || (time_h > 0.0 && self.max_realtime_h > time_h)
            || self.retry_mem_requested_gib > mem_gib
            || self.retry_cpus_requested > cpus
            || (time_h > 0.0 && self.retry_time_limit_h > time_h)
            || self.killed_tasks > 0
    }
}

/// A task's request group: its own request on a first attempt with both `cpus` and `memory`.
/// Bit patterns keep the key exact and orderable.
type GroupKey = (u64, u64, u64);

fn first_attempt_key(t: &Task) -> Option<GroupKey> {
    if t.attempt.unwrap_or(1) != 1 {
        return None;
    }
    let (c, m) = (t.cpus?, t.memory?);
    Some((c.to_bits(), m.to_bits(), t.time_s.unwrap_or(0.0).to_bits()))
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
    /// The CPU-hours and GiB-hours the cost is made of (requested where known, used otherwise),
    /// so that the same run can be priced at any rate: `priced_cpu_h × cpu_hour + priced_gib_h ×
    /// gib_hour` equals `total_cost` at the rates it was analysed with.
    pub priced_cpu_h: f64,
    pub priced_gib_h: f64,
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
    // A retry runs at an escalated request, so it joins the group of the first attempt with the
    // same task name; Nextflow keeps the name across attempts.
    let mut first_keys: HashMap<(&str, &str), BTreeSet<GroupKey>> = HashMap::new();
    for t in tasks {
        if let Some(k) = first_attempt_key(t) {
            first_keys
                .entry((t.process.as_str(), t.name.as_str()))
                .or_default()
                .insert(k);
        }
    }
    let mut groups: BTreeMap<&str, BTreeMap<GroupKey, GroupUsage>> = BTreeMap::new();
    let mut ungrouped: BTreeMap<&str, GroupUsage> = BTreeMap::new();

    for t in tasks {
        let hours = t.realtime_s.unwrap_or(0.0) / 3600.0;
        let cpus_req = t.cpus;
        let mem_req_gib = t.memory.map(|b| b / GIB);
        let has_metrics = t.pct_cpu.is_some() || t.peak_rss.is_some();
        // Used and requested are compared only over tasks that carry both, per resource.
        let cpu_paired = cpus_req.is_some() && t.pct_cpu.is_some();
        let mem_paired = mem_req_gib.is_some() && t.peak_rss.is_some();
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
        // A kill signal, or (AWS Batch records no exit code for a job it stops at its timeout)
        // a failure with no exit code that ran to within 10% of its time limit.
        let signal = t.exit.is_some_and(|e| (130..=145).contains(&e) || e == 104);
        let timed_out = t.exit.is_none()
            && matches!((t.realtime_s, t.time_s), (Some(r), Some(l)) if l > 0.0 && r >= 0.9 * l);
        let killed = t.status == "FAILED" && (signal || timed_out);
        if killed {
            p.killed_tasks += 1;
        }
        if t.succeeded() && t.pct_cpu.is_some() && t.peak_rss.is_some() {
            p.completed_with_metrics += 1;
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

        // Requested resources: what the scheduler had to reserve, hence what is priced. Each
        // resource is priced on its request where there is one, and on what the task used where
        // there is not (a default trace; a process without a `memory` directive), which makes
        // that part a floor.
        if cpus_req.is_none() || mem_req_gib.is_none() {
            run.tasks_without_requests += 1;
        }
        if let (Some(c), Some(m)) = (cpus_req, mem_req_gib) {
            p.has_requests = true;
            p.max_cpus_requested = p.max_cpus_requested.max(c);
            p.max_mem_requested_gib = p.max_mem_requested_gib.max(m);
            if t.attempt.unwrap_or(1) == 1 {
                p.base_cpus_requested = p.base_cpus_requested.max(c);
                p.base_mem_requested_gib = p.base_mem_requested_gib.max(m);
            } else {
                p.retry_cpus_requested = p.retry_cpus_requested.max(c);
                p.retry_mem_requested_gib = p.retry_mem_requested_gib.max(m);
            }
        }
        if let Some(tl) = t.time_s {
            if t.attempt.unwrap_or(1) == 1 {
                p.base_time_limit_h = p.base_time_limit_h.max(tl / 3600.0);
            } else {
                p.retry_time_limit_h = p.retry_time_limit_h.max(tl / 3600.0);
            }
        }
        if let Some(c) = cpus_req {
            p.cpu_h_requested += c * hours;
        }
        if let Some(m) = mem_req_gib {
            p.gib_h_requested += m * hours;
        }

        let key = first_attempt_key(t).or_else(|| {
            match first_keys.get(&(t.process.as_str(), t.name.as_str())) {
                Some(keys) if keys.len() == 1 => keys.first().copied(),
                _ => None,
            }
        });
        let u = match key {
            Some(k) => groups
                .entry(t.process.as_str())
                .or_default()
                .entry(k)
                .or_default(),
            None => ungrouped.entry(t.process.as_str()).or_default(),
        };
        u.tasks += 1;
        u.realtime_h += hours;
        u.max_realtime_h = u.max_realtime_h.max(hours);
        if killed {
            u.killed_tasks += 1;
        }
        if t.peak_rss.is_some() {
            u.tasks_with_rss += 1;
            u.max_rss_gib = u.max_rss_gib.max(rss_gib);
        }
        if t.attempt.unwrap_or(1) > 1 {
            u.retried_tasks += 1;
            if let (Some(c), Some(m)) = (cpus_req, mem_req_gib) {
                u.retry_cpus_requested = u.retry_cpus_requested.max(c);
                u.retry_mem_requested_gib = u.retry_mem_requested_gib.max(m);
            }
            if let Some(tl) = t.time_s {
                u.retry_time_limit_h = u.retry_time_limit_h.max(tl / 3600.0);
            }
        }
        let cpu_part = cpus_req.map_or(cpu_h_used, |c| c * hours);
        let mem_part = mem_req_gib.map_or(gib_h_used, |m| m * hours);
        let cost = cpu_part * rates.cpu_hour + mem_part * rates.gib_hour;
        run.priced_cpu_h += cpu_part;
        run.priced_gib_h += mem_part;
        let cpu_h_req_metered = if cpu_paired { cpu_part } else { 0.0 };
        let gib_h_req_metered = if mem_paired { mem_part } else { 0.0 };
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

    for (process, keyed) in groups {
        let p = by_proc
            .get_mut(process)
            .expect("every grouped task's process is counted");
        p.groups = keyed
            .into_iter()
            .map(|((c, m, tl), usage)| RequestGroup {
                cpus: f64::from_bits(c),
                memory_gib: f64::from_bits(m) / GIB,
                time_limit_h: f64::from_bits(tl) / 3600.0,
                usage,
            })
            .collect();
        p.groups.sort_by(|a, b| {
            (a.memory_gib, a.cpus, a.time_limit_h)
                .partial_cmp(&(b.memory_gib, b.cpus, b.time_limit_h))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    for (process, usage) in ungrouped {
        if let Some(p) = by_proc.get_mut(process) {
            p.ungrouped = usage;
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
    /// Set when some task used more than the first attempt's request (its peak memory or its
    /// run time exceeds it), was retried at a larger request, or was killed (exit 130–145 or
    /// 104, or no exit code within 10% of its time limit). Shrinking such a process could make
    /// those tasks fail, so it is left alone. The process relies on the pipeline's retry
    /// escalation, so no change is recommended and nothing is written to the config fragment.
    pub needs_escalation: bool,
    /// The longest task's run time, in hours.
    #[serde(default)]
    pub longest_h: f64,
    /// Set when the time rule (1.5× the longest task) wants more than the current limit, so the
    /// limit is kept although a task only a little longer than the longest would hit it. This
    /// tool never raises a request; it says so instead.
    #[serde(default)]
    pub time_tight: bool,
    /// Requests (`cpus`, `memory`, `time`) that differ between the process's first attempts:
    /// the pipeline sets them per task. A value in the fragment would apply to every task and
    /// raise the smaller requests, so these are kept and not written.
    #[serde(default)]
    pub varied: Vec<String>,
    /// One entry per request group when the process has more than one, each sized against its
    /// own request.
    #[serde(default)]
    pub groups: Vec<GroupRecommendation>,
}

/// Memory for one request group, sized against that group's own request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupRecommendation {
    pub cpus: f64,
    pub memory_gib: f64,
    pub time_limit_h: f64,
    pub tasks: usize,
    pub retried_tasks: usize,
    /// Largest memory a retry in this group ran at; 0 without retries.
    pub retry_mem_requested_gib: f64,
    pub killed_tasks: usize,
    /// Largest `peak_rss` in the group, retries included; `None` when no task recorded one.
    pub peak_rss_gib: Option<f64>,
    /// The group's peak × margin, rounded as for a process; the request itself when the group
    /// needs its escalation or has no peak.
    pub mem_new_gib: f64,
    pub needs_escalation: bool,
    /// The group's run time re-priced at `mem_new_gib`. Not part of the process's saving: the
    /// fragment cannot write a value per group.
    pub saving: f64,
}

/// Recommend per-process requests: observed peak times a safety margin, rounded to sane units,
/// never below 1 CPU / 1 GiB unless the current request is lower, and never *above* the current
/// request (this tool shrinks over-allocation; it does not diagnose OOM kills, which show up as
/// failed tasks instead). "Current" is the first-attempt request, before retry escalation. A
/// process is sized only when at least half of its tasks completed with both `%cpu` and
/// `peak_rss`; [`not_sized`] names the ones that were not.
pub fn recommend(run: &RunStats, rates: &Rates, margin: f64) -> Vec<Recommendation> {
    let mut out = Vec::new();
    for p in &run.processes {
        // No requests: nothing to shrink. No metrics: no observed peak to shrink to, and a
        // recommendation of "1 CPU / 1 GiB" from a zero would be wrong and dangerous.
        if !p.has_requests || p.tasks == 0 || p.tasks_with_metrics == 0 {
            continue;
        }
        // Too few observations to size from: fewer than half of the process's tasks completed
        // with both metrics (a run stopped early aborts most of them).
        if p.completed_with_metrics * 2 < p.tasks {
            continue;
        }
        let base = |b: f64, max: f64| if b > 0.0 { b } else { max };
        let cpus_now = base(p.base_cpus_requested, p.max_cpus_requested);
        let mem_now = base(p.base_mem_requested_gib, p.max_mem_requested_gib);
        let time_now = base(p.base_time_limit_h, p.max_time_limit_h);
        // A request the pipeline sets per task is kept: one value would raise the smaller ones.
        let grouped = p.groups.len() > 1;
        let varies = |f: fn(&RequestGroup) -> f64| {
            grouped && p.groups.iter().any(|g| f(g) != f(&p.groups[0]))
        };
        let (cpus_vary, mem_vary, time_vary) = (
            varies(|g| g.cpus),
            varies(|g| g.memory_gib),
            varies(|g| g.time_limit_h),
        );
        let varied: Vec<String> = [
            ("cpus", cpus_vary),
            ("memory", mem_vary),
            ("time", time_vary),
        ]
        .into_iter()
        .filter(|&(_, v)| v)
        .map(|(name, _)| name.to_string())
        .collect();
        // Both metrics were observed here: the completion check above requires them.
        let cpus_new = if cpus_vary {
            cpus_now.ceil() as u32
        } else {
            ((p.max_cpu_used_cores * margin).ceil() as u32)
                .max(1)
                .min(cpus_now.ceil() as u32)
        };
        let mem_new = if mem_vary {
            mem_now
        } else {
            round_mem_gib(p.max_rss_gib * margin).max(1.0).min(mem_now)
        };
        let time_wanted = p.max_realtime_h * margin.max(1.5);
        let time_new = if time_now <= 0.0 {
            0.0
        } else if time_vary {
            time_now
        } else {
            round_time_h(time_wanted).min(time_now)
        };
        // A fixed value replaces the pipeline's retry escalation, so a process whose tasks used
        // more than the first attempt's request (peak memory or run time) is left alone: writing the
        // first-attempt value back would make those tasks fail on every attempt.
        // A retry at a larger request shows the escalation in use even when the failed first
        // attempt recorded no metrics (a task killed for memory often records none).
        let needs_escalation = if grouped {
            // Each group against its own request (a 16 GB task retried at 32 GB needs its
            // escalation though another group asks for 96 GB); a task tied to no group against
            // the smallest request, which is the cautious reading.
            let least = |f: fn(&RequestGroup) -> f64| {
                p.groups
                    .iter()
                    .map(f)
                    .filter(|&x| x > 0.0)
                    .fold(f64::INFINITY, f64::min)
            };
            p.groups.iter().any(|g| {
                g.usage
                    .needs_more_than(g.cpus, g.memory_gib, g.time_limit_h)
            }) || p.ungrouped.needs_more_than(
                least(|g| g.cpus),
                least(|g| g.memory_gib),
                least(|g| g.time_limit_h),
            ) || p.killed_tasks > 0
        } else {
            (p.tasks_with_rss > 0 && p.max_rss_gib > mem_now)
                || (time_now > 0.0 && p.max_realtime_h > time_now)
                || p.retry_mem_requested_gib > mem_now
                || p.retry_cpus_requested > cpus_now
                || (time_now > 0.0 && p.retry_time_limit_h > time_now)
                || p.killed_tasks > 0
        };
        let groups: Vec<GroupRecommendation> = if grouped {
            p.groups
                .iter()
                .map(|g| {
                    let u = &g.usage;
                    let escalates = u.needs_more_than(g.cpus, g.memory_gib, g.time_limit_h);
                    let mem_new_gib = if escalates || u.tasks_with_rss == 0 {
                        g.memory_gib
                    } else {
                        round_mem_gib(u.max_rss_gib * margin)
                            .max(1.0)
                            .min(g.memory_gib)
                    };
                    GroupRecommendation {
                        cpus: g.cpus,
                        memory_gib: g.memory_gib,
                        time_limit_h: g.time_limit_h,
                        tasks: u.tasks,
                        retried_tasks: u.retried_tasks,
                        retry_mem_requested_gib: u.retry_mem_requested_gib,
                        killed_tasks: u.killed_tasks,
                        peak_rss_gib: (u.tasks_with_rss > 0).then_some(u.max_rss_gib),
                        mem_new_gib,
                        needs_escalation: escalates,
                        saving: (g.memory_gib - mem_new_gib) * u.realtime_h * rates.gib_hour,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
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
                longest_h: p.max_realtime_h,
                time_tight: false,
                varied,
                groups,
            });
            continue;
        }
        // Saving is estimated by re-pricing the same runtime at the new allocation.
        let saving = if grouped {
            // Each group at its own request; tasks tied to no group are left out.
            p.groups
                .iter()
                .map(|g| {
                    let h = g.usage.realtime_h;
                    (g.cpus - f64::from(cpus_new)).max(0.0) * h * rates.cpu_hour
                        + (g.memory_gib - mem_new).max(0.0) * h * rates.gib_hour
                })
                .sum()
        } else {
            let new_cost = (cpus_new as f64) * p.realtime_h * rates.cpu_hour
                + mem_new * p.realtime_h * rates.gib_hour;
            (p.cost - new_cost).max(0.0)
        };
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
            longest_h: p.max_realtime_h,
            time_tight: !time_vary && time_now > 0.0 && time_wanted > time_now,
            varied,
            groups,
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

/// Processes with requests that [`recommend`] did not size because fewer than half of their
/// tasks completed with both `%cpu` and `peak_rss` (including processes with no metrics at all).
pub fn not_sized(run: &RunStats) -> Vec<&str> {
    run.processes
        .iter()
        .filter(|p| p.has_requests && p.completed_with_metrics * 2 < p.tasks)
        .map(|p| p.process.as_str())
        .collect()
}

/// The `nextflow.config` fragment for these recommendations: one `withName` block per process
/// with a positive saving. Values are written exactly (rounded down to whole MB or seconds where
/// they are not whole GB or hours), so the fragment never asks for more than the current request.
pub fn render_config(recs: &[Recommendation]) -> String {
    let mut s = String::from(
        "// Generated by nf-audit. Review before use; apply with `-c nf-audit.config`.\n\
         // Only lowered values are written. Each one replaces the pipeline's setting for that\n\
         // resource, including any retry escalation (e.g. `memory = { 8.GB * task.attempt }`),\n\
         // so a task that runs out retries at the same size. Unwritten values are left as they are.\n\
         process {\n",
    );
    for x in recs {
        if x.needs_escalation {
            continue;
        }
        // Only the values that go down are written; anything left unchanged keeps whatever the
        // pipeline sets, including its retry escalation.
        let mut lines = Vec::new();
        if f64::from(x.cpus_new) < x.cpus_now {
            lines.push(format!("        cpus   = {}", x.cpus_new));
        }
        if x.mem_new_gib < x.mem_now_gib {
            lines.push(format!("        memory = '{}'", nf_memory(x.mem_new_gib)));
        }
        if x.time_new_h > 0.0 && x.time_new_h < x.time_now_h {
            lines.push(format!("        time   = '{}'", nf_duration(x.time_new_h)));
        }
        if x.time_tight {
            lines.push(format!(
                "        // time: longest task {:.1} h of the {} limit; consider raising it",
                x.longest_h,
                fmt_time_h(x.time_now_h)
            ));
        }
        if lines.is_empty() {
            continue;
        }
        s.push_str(&format!("    withName: '{}' {{\n", x.process));
        for l in lines {
            s.push_str(&l);
            s.push('\n');
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
            longest_h: 0.0,
            time_tight: false,
            varied: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// The fragment must never round a request up past the current one, or down to zero.
    #[test]
    fn config_values_are_exact_and_never_above_the_request() {
        let cfg = |r: Recommendation| render_config(&[r]);
        assert!(cfg(rec(1.0, 0.5, 1.0, 1.0)).contains("memory = '512.MB'"));
        assert!(cfg(rec(4.0, 3.5, 1.0, 1.0)).contains("memory = '3584.MB'"));
        assert!(cfg(rec(3.0, 2.5, 1.0, 1.0)).contains("memory = '2560.MB'"));
        assert!(cfg(rec(72.0, 12.0, 16.0, 16.0)).contains("memory = '12.GB'"));
        assert!(cfg(rec(1.0, 1.0, 2.0, 1.5)).contains("time   = '90.m'"));
        assert!(cfg(rec(1.0, 1.0, 16.0, 15.0)).contains("time   = '15.h'"));
        assert!(cfg(rec(1.0, 1.0, 1.0, 0.25)).contains("time   = '15.m'"));
        assert!(cfg(rec(1.0, 1.0, 0.02, 0.01)).contains("time   = '36.s'"));
        // Unchanged values are not written, so the pipeline's own setting (and escalation) stays.
        let same = cfg(rec(8.0, 8.0, 4.0, 4.0));
        assert!(
            same.contains("cpus   = 1")
                && !same.contains("memory = '")
                && !same.contains("time   = '")
        );
        // A lower time limit alone saves nothing (cost follows run time) but is still written.
        let mut time_only = rec(1.0, 1.0, 16.0, 2.0);
        time_only.cpus_now = 1.0;
        time_only.saving = 0.0;
        assert!(render_config(&[time_only]).contains("time   = '2.h'"));
        let mut kept = rec(1.0, 0.5, 1.0, 0.5);
        kept.needs_escalation = true;
        assert!(!render_config(&[kept]).contains("withName"));
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
    fn repricing_matches_a_fresh_analysis() {
        let tasks = fixtures();
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        for r in Rates::PRESETS {
            let fresh = analyse(&tasks, &r).total_cost;
            assert!((r.price(&run) - fresh).abs() < 1e-9, "{}", r.name);
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
        // Every process has requests and none was sized, so every one is named as not sized.
        assert_eq!(not_sized(&run).len(), run.processes.len());
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
        assert!(!run.processes.is_empty());
        for p in &run.processes {
            assert_eq!(p.mem_efficiency(), None, "{}", p.process);
            assert!(p.cpu_efficiency().is_some());
        }
        // No peak memory anywhere: nothing is sized, and every process is named as unsized.
        assert!(recommend(&run, &Rates::SEQERA_COMPUTE, 1.25).is_empty());
        assert_eq!(not_sized(&run).len(), run.processes.len());
    }

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

    /// rnaseq 3.22.0 `star_rsem`: RSEM's longest task ran 10.9 h of a 16 h limit. 1.5× that is
    /// past the limit, so the limit stays, and the recommendation says it is tight.
    fn one_process_with_longest(longest_h: f64, limit_h: f64) -> Vec<Task> {
        let mut tasks = fixtures();
        let p0 = tasks[0].process.clone();
        tasks.retain(|t| t.process == p0);
        let (cpus, memory) = (tasks[0].cpus, tasks[0].memory);
        for (i, t) in tasks.iter_mut().enumerate() {
            // One clean attempt each at one request, so nothing reads as escalation.
            t.status = "COMPLETED".into();
            t.exit = Some(0);
            t.attempt = Some(1);
            (t.cpus, t.memory) = (cpus, memory);
            t.peak_rss = Some(1.0 * GIB);
            t.time_s = Some(limit_h * 3600.0);
            t.realtime_s = Some(if i == 0 { longest_h } else { 0.5 } * 3600.0);
        }
        tasks
    }

    #[test]
    fn a_time_limit_close_to_the_longest_task_is_flagged() {
        let run = analyse(
            &one_process_with_longest(10.94, 16.0),
            &Rates::SEQERA_COMPUTE,
        );
        let r = &recommend(&run, &Rates::SEQERA_COMPUTE, 1.25)[0];
        assert!(!r.needs_escalation);
        assert!(r.time_tight, "{r:?}");
        assert_eq!((r.time_now_h, r.time_new_h), (16.0, 16.0));
        assert!((r.longest_h - 10.94).abs() < 1e-9);
        let cfg = render_config(std::slice::from_ref(r));
        assert!(
            cfg.contains("// time: longest task 10.9 h of the 16h limit; consider raising it"),
            "{cfg}"
        );
        // A comment is not a value: no `time =` line for a limit that was not lowered.
        assert!(!cfg.contains("time   ="), "{cfg}");
    }

    #[test]
    fn a_time_limit_with_headroom_is_not_flagged() {
        let run = analyse(
            &one_process_with_longest(10.0, 16.0),
            &Rates::SEQERA_COMPUTE,
        );
        let r = &recommend(&run, &Rates::SEQERA_COMPUTE, 1.25)[0];
        assert!(!r.time_tight, "{r:?}");
        assert!(r.time_new_h < 16.0);
        assert!(!render_config(std::slice::from_ref(r)).contains("consider raising"));
    }

    /// A task that ran past the limit is an escalation case, reported as such, not as tight.
    #[test]
    fn a_limit_already_exceeded_is_escalation_not_tight() {
        let run = analyse(
            &one_process_with_longest(17.0, 16.0),
            &Rates::SEQERA_COMPUTE,
        );
        let r = &recommend(&run, &Rates::SEQERA_COMPUTE, 1.25)[0];
        assert!(r.needs_escalation);
        assert!(!r.time_tight);
    }

    /// A report task with `cpus` but no `memory` (a process without a `memory` directive): the
    /// CPU part is priced on the request, the memory part on use.
    #[test]
    fn a_missing_request_is_priced_per_resource() {
        let mut t = fixtures().remove(0);
        t.cpus = Some(8.0);
        t.memory = None;
        t.realtime_s = Some(3600.0);
        t.pct_cpu = Some(50.0);
        t.peak_rss = Some(2.0 * GIB);
        let r = Rates::SEQERA_COMPUTE;
        let run = analyse(&[t], &r);
        let want = 8.0 * r.cpu_hour + 2.0 * r.gib_hour;
        assert!((run.total_cost - want).abs() < 1e-9, "{}", run.total_cost);
        assert_eq!(run.tasks_without_requests, 1);
        assert_eq!(run.cpu_h_requested, 8.0);
        assert!((run.processes[0].cpu_efficiency().unwrap() - 0.5 / 8.0).abs() < 1e-9);
    }

    /// First attempts killed without metrics (exit 137) and retried at a larger request: the
    /// escalation is in use even though no peak shows it.
    #[test]
    fn a_retry_at_a_larger_request_counts_as_escalation() {
        let mut tasks = fixtures();
        let p0 = tasks[0].process.clone();
        let mut killed = tasks[0].clone();
        killed.hash = "kk/1".into();
        killed.attempt = Some(1);
        killed.status = "FAILED".into();
        killed.exit = Some(137);
        killed.pct_cpu = None;
        killed.peak_rss = None;
        let mut retry = killed.clone();
        retry.hash = "kk/2".into();
        retry.attempt = Some(2);
        retry.status = "COMPLETED".into();
        retry.memory = retry.memory.map(|m| m * 2.0);
        retry.pct_cpu = Some(90.0);
        retry.peak_rss = Some(0.1 * GIB);
        tasks.push(killed.clone());
        tasks.push(retry);
        // AWS Batch stops a job at its timeout with no exit code; its run time can end just short
        // of the limit because the limit also covers staging.
        let mut stopped = tasks[0].clone();
        stopped.hash = "tt/1".into();
        stopped.status = "FAILED".into();
        stopped.exit = None;
        stopped.time_s = Some(16.0 * 3600.0);
        stopped.realtime_s = Some(15.9 * 3600.0);
        stopped.pct_cpu = None;
        stopped.peak_rss = None;
        let mut with_stop = fixtures();
        with_stop.push(stopped);
        let killed_in = |ts: &[Task]| {
            analyse(ts, &Rates::SEQERA_COMPUTE)
                .processes
                .iter()
                .find(|q| q.process == p0)
                .unwrap()
                .killed_tasks
        };
        assert_eq!(killed_in(&with_stop), killed_in(&fixtures()) + 1);
        // Killed and never retried (nf-core's PRESEQ_LCEXTRAP ignores its errors) is enough.
        let mut only_killed = fixtures();
        only_killed.push(killed);
        let run1 = analyse(&only_killed, &Rates::SEQERA_COMPUTE);
        let rec1 = recommend(&run1, &Rates::SEQERA_COMPUTE, 1.25);
        assert!(
            rec1.iter()
                .find(|r| r.process == p0)
                .unwrap()
                .needs_escalation
        );
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        let rec = recommend(&run, &Rates::SEQERA_COMPUTE, 1.25);
        let r = rec.iter().find(|r| r.process == p0).unwrap();
        assert!(r.needs_escalation);
        assert!(!render_config(&rec).contains(&format!("'{p0}'")));
    }

    /// One process whose pipeline sets memory per task: two first attempts at 96 GB that peak at
    /// 60 GB, and two at 16 GB that peak at 10 GB. 12 CPUs with about one core used, a 16 h limit,
    /// 1 h each.
    fn two_groups() -> Vec<Task> {
        (0..4)
            .map(|i| {
                let big = i < 2;
                Task {
                    hash: format!("g{i}/1"),
                    name: format!("P:SPLIT (s{i})"),
                    process: "P:SPLIT".into(),
                    tag: Some(format!("s{i}")),
                    status: "COMPLETED".into(),
                    exit: Some(0),
                    attempt: Some(1),
                    realtime_s: Some(3600.0),
                    pct_cpu: Some(100.0),
                    peak_rss: Some(if big { 60.0 } else { 10.0 } * GIB),
                    cpus: Some(12.0),
                    memory: Some(if big { 96.0 } else { 16.0 } * GIB),
                    time_s: Some(16.0 * 3600.0),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn sized(tasks: &[Task]) -> Recommendation {
        let r = Rates::SEQERA_COMPUTE;
        let mut recs = recommend(&analyse(tasks, &r), &r, 1.25);
        assert_eq!(recs.len(), 1);
        recs.remove(0)
    }

    /// A value in the fragment applies to every task, so where the pipeline sets memory per task
    /// one value would raise the smaller requests (16 GB to 76 GB here). Memory is left out; the
    /// CPUs, the same on every task, are still lowered, and the saving prices only that.
    #[test]
    fn a_request_that_varies_by_task_is_not_written() {
        let rec = sized(&two_groups());
        assert!(!rec.needs_escalation);
        assert_eq!(rec.mem_new_gib, rec.mem_now_gib);
        let cfg = render_config(std::slice::from_ref(&rec));
        assert!(!cfg.contains("memory = '"), "{cfg}");
        assert!(cfg.contains("cpus   = 2"), "{cfg}");
        // (12 - 2) CPUs × 4 task-hours × $0.10.
        assert!((rec.saving - 4.0).abs() < 1e-9, "{}", rec.saving);
    }

    /// Each group gets its own peak, retries and suggestion. A process with one request has one
    /// group, nothing varied and no per-group rows: it is sized as before.
    #[test]
    fn each_group_is_sized_against_its_own_request() {
        let rec = sized(&two_groups());
        assert_eq!(rec.varied, ["memory"]);
        let g: Vec<(f64, Option<f64>, f64)> = rec
            .groups
            .iter()
            .map(|g| (g.memory_gib, g.peak_rss_gib, g.mem_new_gib))
            .collect();
        // 10 × 1.25 = 12.5 -> 14 GB; 60 × 1.25 = 75 -> 80 GB (above 64 GB, in 8 GB steps).
        assert_eq!(g, [(16.0, Some(10.0), 14.0), (96.0, Some(60.0), 80.0)]);
        // (16 - 14) GB × 2 h × $0.025 and (96 - 80) GB × 2 h × $0.025.
        assert!((rec.groups[0].saving - 0.1).abs() < 1e-9);
        assert!((rec.groups[1].saving - 0.8).abs() < 1e-9);

        let r = Rates::SEQERA_COMPUTE;
        let run = analyse(&fixtures(), &r);
        assert!(run
            .processes
            .iter()
            .filter(|p| p.has_requests)
            .all(|p| p.groups.len() == 1 && p.ungrouped.tasks == 0));
        assert!(recommend(&run, &r, 1.25)
            .iter()
            .all(|x| x.varied.is_empty() && x.groups.is_empty()));
    }

    /// Each task is checked against its own group's request. A 16 GB task retried at 32 GB, or
    /// peaking above 16 GB, needs the escalation although both stay under the 96 GB group's
    /// request; so does a retry that cannot be tied to one group, against the smallest request.
    #[test]
    fn escalation_is_judged_against_the_tasks_own_group() {
        let retried = |name: &str| {
            let mut tasks = two_groups();
            tasks[3].status = "FAILED".into();
            tasks[3].exit = Some(1);
            (tasks[3].pct_cpu, tasks[3].peak_rss) = (None, None);
            let mut retry = tasks[3].clone();
            retry.hash = "g3/2".into();
            retry.name = name.into();
            retry.attempt = Some(2);
            retry.status = "COMPLETED".into();
            retry.exit = Some(0);
            retry.memory = Some(32.0 * GIB);
            retry.pct_cpu = Some(100.0);
            retry.peak_rss = Some(12.0 * GIB);
            tasks.push(retry);
            tasks
        };
        let rec = sized(&retried("P:SPLIT (s3)"));
        assert!(rec.needs_escalation);
        let g = &rec.groups;
        assert_eq!(
            (
                g[0].retried_tasks,
                g[0].retry_mem_requested_gib,
                g[0].mem_new_gib
            ),
            (1, 32.0, 16.0)
        );
        assert_eq!(
            (g[0].needs_escalation, g[1].needs_escalation),
            (true, false)
        );
        assert!(sized(&retried("P:SPLIT (unmatched)")).needs_escalation);
        let mut over = two_groups();
        over[2].peak_rss = Some(20.0 * GIB);
        assert!(sized(&over).needs_escalation);
        // The same retry at the task's own request is not escalation.
        let mut same = retried("P:SPLIT (s3)");
        same[4].memory = Some(16.0 * GIB);
        assert!(!sized(&same).needs_escalation);
    }

    /// A process where most tasks never completed (a run stopped early) is not sized from the
    /// one or two that did.
    #[test]
    fn too_few_completed_tasks_get_no_recommendation() {
        let mut tasks = fixtures();
        let p0 = tasks[0].process.clone();
        let n = tasks.iter().filter(|t| t.process == p0).count();
        let mut aborted = 0;
        for t in tasks.iter_mut().filter(|t| t.process == p0).skip(1) {
            t.status = "ABORTED".into();
            t.pct_cpu = None;
            t.peak_rss = None;
            aborted += 1;
        }
        assert!(aborted * 2 > n);
        let run = analyse(&tasks, &Rates::SEQERA_COMPUTE);
        assert!(!recommend(&run, &Rates::SEQERA_COMPUTE, 1.25)
            .iter()
            .any(|r| r.process == p0));
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
