# nf-audit

Where the CPU-hours and dollars of a Nextflow run actually go.

`nf-audit` reads the two files every Nextflow run already writes, `execution_trace_*.txt` and `execution_report_*.html`, and turns them into a per-process cost breakdown, an over-allocation figure, a retry-waste figure, and a right-sized `nextflow.config` fragment. It runs offline, on any executor (AWS Batch, Google Batch, Slurm, Kubernetes, local), on runs that never went through Seqera Platform. A `compare` mode lines up many runs, for example every release of a pipeline, and shows which processes carry the cost over time.

```
nf-audit inspect  results/pipeline_info/execution_report_2024-09-16_16-33-21.html
nf-audit analyze  --trace  results/pipeline_info/execution_trace_2024-09-16_16-33-21.txt \
                  --report results/pipeline_info/execution_report_2024-09-16_16-33-21.html \
                  --rates seqera-compute --config-out nf-audit.config > report.md
nf-audit compare  data/rnaseq/*/aligner_star_salmon/pipeline_info/execution_report_*.html > releases.md
```

## What it reads

**The report HTML matters more than the trace.** nf-core pipelines enable `trace {}` without custom `fields`, so the TSV trace has only the 14 default columns: what each task *used* (`%cpu`, `peak_rss`, `realtime`) but not what it *requested* (`cpus`, `memory`, `time`). Billing follows the request. The execution report embeds a `window.data` JSON blob with the full per-task record including the requests, so `--report` is what turns "usage" into "cost". When both are given, the report's exact values (milliseconds, bytes) replace the trace's rounded ones and any task present in only one file is kept.

Trace-only input still works; the report then prices *used* resources and labels the total as a floor.

**Some runs carry no usage metrics at all.** Nextflow collects `%cpu` and `peak_rss` with `ps` inside the task container; a container without procps, and some Fusion/Wave combinations, leave every one of them `-`. nf-audit then still prices the run exactly (cost follows the request) but reports efficiency, waste and right-sizing as unknown rather than as zero. `inspect` tells you up front how many tasks are metered. Of the nf-core/rnaseq megatest runs, the June 2025 ones (Nextflow 25.04.2, Fusion 2.4) are unmetered; runs before and after are fine.

## Cost model

Cost per task = `cpus_requested × realtime_h × cpu_hour + memory_requested_GiB × realtime_h × gib_hour`.

Presets:

- `seqera-compute`: $0.10 per CPU-hour plus $0.025 per GiB-hour, Seqera Compute's published list price (seqera.io/pricing, Sept 2026). This is the one preset that is exactly the model above, so its output is what a run *would bill* on that service.
- `aws-m5-ondemand`, `aws-m5-spot`: the m5 family split into per-vCPU and per-GiB components ($0.032 + $0.004, and the same at a 65% spot discount). Approximations; override with `--cpu-hour` / `--gib-hour` for your region and family.

Two numbers price any executor that bills on allocation. The presets differ by 13× on the same run, so quote the preset with the number; the process ranking and the unused share do not depend on it.

"Unused" is the dollar value of allocation that was never used (requested minus used, integrated over run time), computed over the tasks that have usage metrics. "Failed cost" is what was spent on attempts that did not complete.

### How this differs from Seqera Platform's estimate

Platform's per-task estimate is `VM hourly rate × max(task cpus / VM cpus, task memory / VM memory) × (complete − start)`: it charges the dominant resource of the actual instance type at that instance's price, and it uses wall time including file staging (docs.seqera.io/platform-cloud/monitoring/cloud-costs). nf-audit charges CPU and memory additively at a flat rate on `realtime`. On the same nf-core/rnaseq 3.15.1 `test_full` run, Platform reported $34.90 on AWS Batch spot with Fusion ($58.40 on plain S3); nf-audit's `seqera-compute` preset gives $79.23 and `aws-m5-spot` gives $6.21. Both bracket the Platform figure for understandable reasons: the list price is a managed-service price, and the additive m5 split ignores that a 72 GB request pins a whole memory-heavy slot. An instance-aware "dominant resource" model is on the roadmap; until then use the preset as a consistent yardstick across runs, not as an invoice.

## Right-sizing

For each process with usage metrics: new request = observed peak × margin (default 1.25), rounded to sane units, never above the current request, never below 1 CPU / 1 GiB. Time limit = max observed run time × 1.5, rounded up. Processes without metrics get no recommendation. The tool shrinks over-allocation; it does not diagnose OOM kills (those appear as failed tasks, look at the `failed` column).

Validate on one real run before rolling out. A process at 100% CPU efficiency slows down if you cut its cores.

## Public data to demo on

nf-core publishes the traces and reports of every full-size AWS test run in a public bucket. Layout changed over time: older rnaseq runs keep one `pipeline_info/` per aligner (`aligner_star_salmon/`, `aligner_star_rsem/`), newer ones a single `pipeline_info/`. The script finds either.

```
scripts/pull-megatests.sh --list rnaseq
scripts/pull-megatests.sh rnaseq results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb   # rnaseq 3.15.1 test_full, 8 ENCODE samples, metered
scripts/pull-megatests.sh rnaseq results-0bb032c1e3b1e1ff0b0a72192b9118fdb5062489   # rnaseq 3.19.0, unmetered, one QUALIMAP failure
scripts/pull-megatests.sh sarek results-dev
```

Anchor to reconcile against: Seqera's published figure for nf-core/rnaseq **3.15.1** `test_full` on AWS Batch (m5/r5, spot and on-demand) is $34.90 with Fusion and $58.40 on plain S3 (docs.seqera.io/platform-cloud/getting-started/rnaseq). That run is Seqera's own, not a megatest, but the megatest 3.15.1 `star_salmon` run has the same profile and input and completed in 5h 8m with 316.9 CPU-hours, which is the number Nextflow itself prints in the report header and which `nf-audit` reproduces exactly.

## Status

v0.1, Sept 2026. Parsers are unit-tested against Nextflow's formatting and have been run against 17 rnaseq megatest releases from 3.1 (Nextflow 21.04) to 3.17 (24.04) plus 3.19.0 (25.04). Two things learned from real files that the synthetic fixtures did not show: the report's own script references `window.data_byprocess`, `window.data.summary` and `window.data.trace` before the assignment, and every task's `script` field carries JavaScript-only `\'` escapes, so the blob is not valid JSON until they are rewritten. Report-JSON conventions follow Nextflow's `ReportObserver.renderJsonData` (every `TraceRecord.FIELDS` entry as a JSON string: memory in bytes, time in milliseconds, `%cpu` as a float, `cpus` as an integer string). A report with more than `maxTasks` tasks (default 10,000) embeds `"trace": null`; nf-audit says so and points you at a trace configured with `trace.fields`.

## Roadmap

- Instance-aware pricing (Seqera's dominant-resource formula against an instance table; `start`/`complete` timestamps instead of `realtime`).
- Retry attribution: cost of spot reclamation vs genuine failures.
- Per-sample cost.
- JSON/CSV output for dashboards.

MIT. Author: Sushant Hona.
