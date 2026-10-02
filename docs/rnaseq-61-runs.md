# Where nf-core/rnaseq's compute goes: 61 AWS test runs, 2021–2026, process by process

*By Sushant Hona ([@OtoYuki](https://github.com/OtoYuki)) — October 2026*

Seqera's documentation prices one run of nf-core/rnaseq **3.15.1** on AWS Batch, on the pipeline's full-size test data: 8 paired-end human RNA-seq samples from the GM12878, K562, MCF7 and H1 cell lines, 132.4 GB of gzipped FASTQ. It cost **$34.90** with the Fusion file system and fast instance storage, and **$58.40** on plain S3 ([docs.seqera.io](https://docs.seqera.io/platform-cloud/getting-started/rnaseq)). That is a useful headline. It does not say which steps the money goes to, or how much of it pays for CPU and memory that were reserved and never used.

nf-core publishes the execution trace and report of its full-size AWS test runs in a public bucket (`s3://nf-core-awsmegatests`). I wrote [`nf-audit`](https://github.com/OtoYuki/nf-audit), a small offline Rust CLI that joins a run's trace with the resource *requests* embedded in its HTML report and prices every task. I pointed it at **61 of those runs**:
- 30 releases on the default `star_salmon` route, 3.1 (May 2021) to 3.26.0 (May 2026);
- 31 releases on the `star_rsem` route, 3.1 to 3.27.0 (23 September 2026).

**Selection rule.** For each release tag and route, I took the last successful run stored under that tag's own `results-<commit>/` prefix in the bucket. On `star_rsem`, where a release has no successful run, I took the last run in which an `RSEM_CALCULATEEXPRESSION` task failed (not counting tasks aborted because something else failed).

**What that leaves out:**
- Runs with no usage data: the two 3.19.0 runs.
- Releases where no run qualifies: 3.11.0, 3.13.0 `star_salmon`, 3.26.0 `star_rsem`.
- Releases with no `test_full` report in the bucket: 3.0, 3.10, 3.20.0, 3.21.0, and 3.27.0 on `star_salmon`.
- Runs stored under other prefixes: a November 2023 re-run of 3.12.0, a run of the release/3.25.0 merge commit, and the `dev`-branch runs, which I come back to in section 3.

**Sources.** Cost tables are nf-audit output. The commands and the exact report behind each column are in [`examples/`](https://github.com/OtoYuki/nf-audit/tree/main/examples). Workflow outcomes and log lines are quoted from the reports themselves. The thread benchmark is my own measurement.

**Units.** Nextflow reports memory in GiB, labelled "GB"; I write GiB.

## What the runs show

1. **Unused allocation stays between 62% and 68% of metered allocated cost in 28 of the 30 `star_salmon` releases.** The exceptions are 3.5 (81%, a `-resume` run) and 3.6 (73%, 13 failed attempts). The allocated cost of a run fell from $138.66 to $59.64 over five years, with steps at changes to how the tests are launched and, in 3.26.0, at nf-core upgrading two processes and resizing one of them. The share paid for idle reservations barely moved.
2. **`QUALIMAP_RNASEQ` requests 6 CPUs and 36 GiB in every release, and no task of it ever averaged more than 1.08 cores.** (In 3.6, two tasks that failed their first attempt were retried at 12 / 72.) It was the second most expensive process in 3.6 and from 3.9 to 3.18, at 18–23% of the run.
3. **In four releases (3.22.1, 3.23.0, 3.24.0, 3.25.0) the `star_rsem` test failed because an `RSEM_CALCULATEEXPRESSION` task reached its 16-hour limit.**
   - In three of the four, the task's stdout ends on a progress line from RSEM's single-threaded parsing step. On my laptop that step handles a chromosome's worth of reads in about 95 seconds.
   - Since STAR moved out of this task (3.21.0), the task has kept a 12-CPU / 72 GiB reservation. In the runs since, completed tasks average a median of 1.04 cores (mean 1.20, range 0.25–5.1) and peak at 9.45 GiB.
4. **nf-audit's requested CPU-hours match the figure Nextflow prints in its own report header on all 61 runs, to within 0.05 CPU-hours (the header rounds to one decimal).** That checks the parsing against Nextflow's own accounting. It does not validate the prices.

## How the cost is computed

A Nextflow run leaves two files in `pipeline_info/`:
- the **trace** (`execution_trace_*.txt`) records what each task *used*: run time, `%cpu`, peak RSS;
- the **report** (`execution_report_*.html`) also embeds what each task *requested*: `cpus`, `memory`, `time`.

nf-core's default trace fields omit the requests, and a batch scheduler places a task by what it requests, not by what it later uses. nf-audit therefore reads the requests from the report and prices each task as:

```
task cost = cpus_requested × realtime_h × rate_cpu  +  memory_requested_GiB × realtime_h × rate_GiB
```

The unused share is the part of that cost not matched by measured use (`%cpu`, peak RSS). Three rate presets are built in:

| preset | per CPU-hour | per GiB-hour | what it models |
|---|---:|---:|---|
| `seqera-compute` | $0.10 | $0.025 | Seqera Compute list price ([seqera.io/pricing](https://seqera.io/pricing/)) |
| `aws-m5-ondemand` | $0.032 | $0.004 | AWS m5.large on-demand, us-east-1 ($0.096/h), split per resource |
| `aws-m5-spot` | $0.0112 | $0.0014 | the same at an assumed 65% spot discount |

Unless stated, dollar figures below use `seqera-compute`. **They are a consistent yardstick across runs, not what AWS billed nf-core for these runs.** The unused share also shifts a little with the preset: 3.15.1 is 66% on `seqera-compute` and 64% on the m5 presets.

## 1. Five years of `star_salmon`: cheaper runs, a steady idle share

| release | tasks | wall | CPU-h requested | cost (`seqera-compute`) | cost (`aws-m5-spot`) | unused | CPU efficiency |
|---|---:|---:|---:|---:|---:|---:|---:|
| 3.1 (May 2021) | 285 | 15h 5m | 554.6 | $138.66 | $10.87 | 67% | 37% |
| 3.8.1 (May 2022) | 285 | 7h 23m | 542.1 | $135.52 | $10.63 | 67% | 35% |
| 3.10.1 (Jan 2023) | 277 | 10h 5m | 480.0 | $120.00 | $9.41 | 63% | 43% |
| 3.11.1 (Mar 2023) | 277 | 4h 19m | 337.2 | $84.30 | $6.61 | 64% | 41% |
| 3.15.1 (Sep 2024) | 283 | 5h 9m | 316.9 | $79.23 | $6.21 | 66% | 41% |
| 3.18.0 (Dec 2024) | 299 | 4h 43m | 316.1 | $79.03 | $6.20 | 67% | 40% |
| 3.22.0 (Nov 2025) | 303 | 5h 11m | 310.7 | $77.66 | $6.09 | 64% | 42% |
| 3.26.0 (May 2026) | 328 | 4h 29m | 246.2 | $59.64 | $4.72 | 62% | 42% |

These are selected rows; all 30 releases are in [`rnaseq-star_salmon-releases.md`](https://github.com/OtoYuki/nf-audit/blob/main/examples/rnaseq-star_salmon-releases.md).
- **Unused share:** 62–68% in 28 of them.
- **3.5:** its successful run is a `-resume` after a failed first attempt. It costs $332.25, over half of it in `RSEQC_TIN`, and is 81% unused.
- **3.6:** 73% unused, with 13 failed attempts.

**Read the cost column with care.** Changes between releases mix pipeline and infrastructure changes:

- **3.10.1 → 3.11.1 ($120.00 → $84.30).** This is the largest drop between two clean runs. (Only 3.5 → 3.6, driven by 3.5's `-resume` outlier, is larger; 3.6 → 3.7, with 13 failed attempts in 3.6, is almost as large.) It coincides with nf-core changing how these tests are launched in 3.11.0 ([#981](https://github.com/nf-core/rnaseq/pull/981)): a new CI workflow, with a renamed compute-environment secret, and the profile going from `test_full,aws_tower` to `test_full_aws`. I don't attribute that drop to either side.
- **Between 3.18.0 and 3.22.0** the launch profile changed again (`test_full_aws` → `test_full`, already in place in the excluded 3.19.0 runs), with Nextflow going from 24.10 to 25.04. Changes across that boundary are equally unattributed. `STAR_ALIGN_IGENOMES` rising from $19.24 (3.18.0) to $24.43 (3.22.0) is one of them.
- **3.25.0 → 3.26.0 ($79.29 → $59.64).** nf-core changed two processes, and in both cases the tool changed too.
  - `TRIMGALORE`: $13.15 → $1.34. It moved to Trim Galore 2.1.0 (#1789) and from 12 CPUs / 72 GiB to 8 CPUs / 1 GiB (#1836, #1841, #1842). Its run time fell from 4.38 h to 1.62 h despite fewer CPUs. How the $11.81 splits between shorter run time and smaller request depends on the order you apply them: time first gives $8.29 / $3.52, request first $2.28 / $9.53.
  - STAR: $23.86 → $16.16. The legacy STAR 2.6.1d pin was dropped for 2.7.11b (#1835); STAR's request stayed at 12 / 72, so its saving is all run time.
  - The two differences sum to $19.51 of the $19.65 net drop. Other processes moved by up to $3.19 (`DUPRADAR`, down) between these two single runs, so the split is approximate.

The unused share is the steadier signal. It held within 62–68% in every clean run, including after nf-core's own TRIMGALORE fix.

## 2. The anchor run: 3.15.1 `star_salmon`

This is the release Seqera's $34.90 refers to. The megatest run of it (283 tasks, 5h 8m 46s wall) breaks down as follows.

| | `seqera-compute` | `aws-m5-ondemand` | `aws-m5-spot` |
|---|---:|---:|---:|
| allocated cost | $79.23 | $17.75 | $6.21 |
| of which unused | $52.08 (66%) | $11.33 (64%) | $3.97 (64%) |

- CPU-hours: 316.9 requested, 130.0 used (41%).
- GiB-hours: 1,901.4 requested, 568.1 used (30%).
- Summed task run time: 50.8 h.

The five most expensive processes:

| process | cost | share | CPU eff. | mem eff. | request | peak use (worst task) |
|---|---:|---:|---:|---:|---|---|
| `STAR_ALIGN_IGENOMES` | $19.28 | 24% | 47% | 50% | 12 CPU / 72 GiB | 6.2 cores / 35.9 GiB |
| `QUALIMAP_RNASEQ` | $18.00 | 23% | 16% | 14% | 6 CPU / 36 GiB | 0.98 cores / 5.3 GiB |
| `TRIMGALORE` | $13.49 | 17% | 61% | 8% | 12 CPU / 72 GiB | 7.5 cores / 6.2 GiB |
| `PICARD_MARKDUPLICATES` | $7.99 | 10% | 22% | 76% | 6 CPU / 36 GiB | 1.3 cores / 29.6 GiB |
| `SALMON_QUANT` (pseudo-alignment) | $3.19 | 4% | 99% | 48% | 6 CPU / 36 GiB | 6.0 cores / 17.4 GiB |

Per sample, the same run allocates $8.55–$10.80. That comes from grouping tasks by the sample name in their tag; 98.8% of the cost carries a sample tag.

### `QUALIMAP_RNASEQ` across all 30 releases

- Its request is 6 CPUs / 36 GiB in every release. In 3.6, two tasks that failed their first attempt were retried at 12 / 72.
- Its most CPU-hungry task never averaged more than **1.08 cores**.
- CPU efficiency stayed at 13–17% throughout.
- Rank and share over time:
  - 3.1–3.4 and 3.7–3.8.1: third most expensive process. In 3.5 it was fourth.
  - 3.6 and 3.9–3.18: second, at 18–23% of the run.
  - From 3.22.0: its run time fell and it dropped to 8–12%.
- Peak memory was 5.1–7.0 GiB from 3.14.0 onward.

This one is cheap to fix from the outside, with no pipeline change (see *Doing something about it*).

## 3. `star_rsem`: a single-threaded step against a 16-hour limit

`--aligner star_rsem` quantifies with RSEM's expectation-maximisation. The route also runs Salmon pseudo-alignment, but RSEM produces its main quantification. On this route `RSEM_CALCULATEEXPRESSION` is the most expensive process in every release I analysed except 3.5, where `RSEQC_TIN` was. What that task contains changed over time:

| releases | what the task does | RSEM share of run | RSEM CPU eff. | RSEM peak memory |
|---|---|---:|---:|---:|
| 3.1 – 3.10.1 | STAR alignment *and* RSEM (`--star`); in 3.5 and 3.6, retried attempts ran at 16 CPU / 128 GiB (the `max_cpus` / `max_memory` caps) | 31–48% | 42–53% | 30.3–32.0 GiB |
| 3.11.1 – 3.17.0 | same RSEM command (image now pulled through Wave) | 64–68% | 17–20% | 30.3 GiB |
| 3.18.0 | same; 4 of 12 attempts exited 143 after ~6.7 h and were retried at 24 CPU / 144 GiB | 86% | 8% | 33.5 GiB |
| 3.22.0, 3.22.2 | RSEM only, on STAR's transcriptome BAM (`--alignments`) | 69–75% | 9–11% | ≤9.45 GiB |
| 3.22.1, 3.23.0 – 3.25.0 | same; **the test run fails**: an RSEM task reaches the 16 h limit | 61–74% | 8–9% | ≤9.45 GiB |
| 3.27.0 | same; passes, after one task needed three attempts (exit 175, the code Fusion Snapshots uses for an unrecoverable error) | 89% | 9% | ≤9.45 GiB |

(The 3.15.1 `star_rsem` report is from a `-resume` run: 195 of its 271 tasks are cached and priced at their recorded run times.)

**3.10.1 → 3.11.1: same RSEM command, different behaviour.** RSEM's script, arguments and base image are identical in both releases. From 3.11.1 the image is served through Wave, and Nextflow went from 22.10.4 to 23.03.0-edge. The jump in share coincides with the change in test infrastructure noted above, and I have not established its cause.

**3.21.0: STAR moved out of the RSEM task.** [PR #1604](https://github.com/nf-core/rnaseq/pull/1604) ("Enable BAM input for RSEM", released in 3.21.0) made STAR run as its own task and feeds RSEM the transcriptome BAM. That decoupling is a sensible design. The RSEM task, however, kept the nf-core module's `process_high` label: 12 CPUs, 72 GiB, 16 h. In 3.22.0, the first run I have after that change, its 8 tasks did this:

| sample | run time | avg. CPU | peak RSS |
|---|---:|---:|---:|
| K562_REP1 | 7h 42m | 102% | 6.7 GiB |
| GM12878_REP1 | 7h 58m | 119% | 7.1 GiB |
| GM12878_REP2 | 8h 5m | 104% | 7.2 GiB |
| H1_REP2 | 9h 5m | 111% | 8.3 GiB |
| K562_REP2 | 9h 29m | 105% | 7.8 GiB |
| MCF7_REP1 | 10h 12m | 89% | 8.5 GiB |
| H1_REP1 | 10h 29m | 99% | 9.4 GiB |
| MCF7_REP2 | 10h 56m | 103% | 9.0 GiB |

- That process is $221.80 of the run's $294.25 (75%).
- The whole `star_rsem` run allocates 3.8× what `star_salmon` does on the same samples in the same release ($77.66), and 84% of its allocated cost goes unused.
- 3.22.2 repeats the pattern: RSEM 99–178% CPU, ≤9.45 GiB, 69% of the run.

In some runs, individual RSEM tasks took much longer, long enough to reach the 16-hour limit. Each workflow's own report says:

| release | workflow result | cause |
|---|---|---|
| 3.22.1 | completed unsuccessfully | `RSEM_CALCULATEEXPRESSION (GM12878_REP2)`: `Job attempt duration exceeded timeout`, after 16.01 h |
| 3.23.0 | completed unsuccessfully | same, `H1_REP1`, 16.05 h |
| 3.24.0 | completed unsuccessfully | same, `GM12878_REP2`, 16.04 h |
| 3.25.0 | completed unsuccessfully | same, `MCF7_REP2`, 16.01 h |
| 3.26.0 | completed unsuccessfully (2 runs) | unrelated: the May run stopped on `SALMON_QUANT`; in the June run no RSEM task started |
| 3.27.0 | completed successfully | `H1_REP2`: 19.4 h at 12 CPU / 72 GiB (exit 175), 4.5 h at 24 / 144 (exit 175), then 9.1 h at 36 / 216 |

**In each failed run:**
- The timed-out attempt recorded no exit status and was not retried, and the run stopped.
- The other running RSEM tasks show as aborted. Most had started about 16 h before the run stopped: in 3.24.0, five of the eight had started 15.8–16.1 h earlier.

**Where the time went.** The last line of each timed-out task's stdout:

| release | task | last stdout line |
|---|---|---|
| 3.22.1 | `GM12878_REP2` | `Parsed 245000000 entries` |
| 3.23.0 | `H1_REP1` | `Parsed 255000000 entries` |
| 3.24.0 | `GM12878_REP2` | `Parsed 73000000 entries` |
| 3.25.0 | `MCF7_REP2` | no stdout; stderr has only an hour of Fusion snapshot signals, so the step can't be told |

`Parsed N entries` is progress output from `rsem-parse-alignments`. In RSEM's code, that step runs before the multi-threaded EM step and is given no thread option. So, as of their last logged line, those three tasks had not reached the EM.

**3.27.0:**
- Wall time was 1 day 10 hours and allocated cost $464.22, 89% of it RSEM.
- 87% of the metered allocated cost ($330 of $379) went unused. The two failed attempts ($85) have no usage data.
- The two failed attempts exited 175, the code Fusion Snapshots emits on an unrecoverable error (nf-core added it to the retry list for that reason, commit c83289f5). So the retries were not about RSEM. Each one still doubled and then tripled the CPUs and memory, because nf-core's resource labels scale with `task.attempt`.
- `MCF7_REP1` completed at 22.6 h and `H1_REP2`'s first attempt ran 19.4 h, both on a 16 h `time`. In 3.25.0 an aborted task had also started 17.0 h before the run stopped. So in 3.25.0 and 3.27.0, tasks ran past 16 h without being stopped by the limit. I don't know why.

### Does RSEM just need to be given its threads?

I reran the 3.22.0 command line locally to find out. Setup:
- Same container and arguments, plus RSEM's `--time`.
- Input: the 7.79 million read pairs that 3.22.0's own STAR run placed on chromosome 1 for GM12878_REP1, streamed from the published BAM and realigned with the pipeline's STAR arguments to a chr1 index.
- One container capped to N CPUs per run, on a laptop with an 8-core / 16-thread i7-11800H and local NVMe (so 12 threads relies on hyper-threading).

| threads | wall | speed-up | avg. CPU | CPUs × wall |
|---:|---:|---:|---:|---:|
| 1 | 1,699 s | 1.00× | 99% | 1,699 CPU·s |
| 2 | 976 s | 1.74× | 185% | 1,952 CPU·s |
| 4 | 574 s | 2.96× | 335% | 2,297 CPU·s |
| 8 | 440 s | 3.86× | 545% | 3,522 CPU·s |
| 12 | 389 s | 4.36× | 757% | 4,673 CPU·s |

- **The results are identical at every thread count** (same sha256 for the gene and isoform tables).
- **The whole task parallelises with diminishing returns.** Four threads reach 68% of the 12-thread speed for about half the reserved CPU-seconds (CPUs × wall).
- **The parse step alone**, at 1 CPU, took 93–98 s over two runs (the first transcribed by hand; see `examples/rsem-threads/`) for the 18 million alignment entries: about 190,000 entries a second. It is single-threaded by RSEM's code; I only ran it at 1 CPU.
- **The timed-out megatest tasks** logged about 4,300 (3.22.1), 4,400 (3.23.0) and 1,300 (3.24.0) entries a second over their 16 hours. Assuming they spent all of it parsing, that is ~40–155× slower than on my laptop.
- **The I/O pattern is similar.** In a separate 12-thread run, RSEM read 21.9× its input (37.4 GB for a 1.71 GB BAM), slightly above the megatest range. The megatest tasks read 17.6–20.7× theirs: their BAM sizes (20.3–25.8 GB) are recorded in each task's script, and one outlier read only 1.3×.

So RSEM can use its cores, and its parsing is fast on local disk. On the test infrastructure the same command averages a median of 1.04 cores over the 45 completed tasks since 3.22.0 (mean 1.20, range 0.25–5.1). There is wild spread between samples in the same run: 3.27.0's MCF7_REP1 ran 22.6 h at 25% CPU, and GM12878_REP1 9.6 h at 511%. I have not established why. The scripts, raw outputs and hashes are in [`examples/rsem-threads/`](https://github.com/OtoYuki/nf-audit/tree/main/examples/rsem-threads).

Two things hold on this evidence:
1. **Memory can come down to 16 GiB.** The peak is 9.45 GiB or less in every completed task across seven releases.
2. **More cores would most likely not have helped these failures.** In the three release-run failures where the step is visible, the task was still in RSEM's single-threaded parse step when the limit hit. A longer time limit would most likely let the test pass (3.27.0's MCF7_REP1 completed at 22.6 h). The underlying question is why parsing is so slow on the test infrastructure.

**The `dev`-branch runs.** `results-dev/aligner_star_rsem/` in the bucket holds 19 full-size `star_rsem` runs of the `dev` branch, from November 2025 to September 2026, stored outside the release prefixes:
- 8 completed.
- 7 stopped on the same 16 h RSEM timeout. In six of those, stdout ends on a `Parsed N entries` line (80–257 million). In the seventh, it ends on `515000000 alignment lines are loaded!`, which RSEM prints while writing its output BAM after the EM step.
- 2 stopped when RSEM exited 255 after 8–10 h, with `Done!` then `Fail to open file …bam!` in stdout.
- 2 were cancelled (SIGTERM) after 3–4.5 h, with all eight RSEM tasks still running.

So slow parsing is the usual pattern, not the only one.

**It also predates the STAR split.** Three more full-size `dev` runs from June 2025, from before #1604, also stopped on the 16 h RSEM timeout. Each ends its stdout on `Parsed N entries` (66–96 million), while the task still ran STAR itself. So the STAR split explains why the request is now oversized, not why parsing is slow.

## Comparing with Seqera's $34.90

nf-audit and Seqera Platform price differently, and neither is "the" cost of a run:
- **Platform** estimates each task's cost as `VM hourly rate × max(task CPUs / VM CPUs, task memory / VM memory) × task runtime`. That is the dominant resource of the actual instance, at that instance's price. By Seqera's own note it doesn't account for storage, network, the head job, or how tasks are mapped to VMs, and is meant "for at-a-glance heuristic purposes" ([docs](https://docs.seqera.io/platform-cloud/monitoring/cloud-costs)).
- **nf-audit** charges CPU and memory additively at a flat rate on `realtime`.

For 3.15.1 the presets give $79.23 (`seqera-compute`), $17.75 (`aws-m5-ondemand`) and $6.21 (`aws-m5-spot`). Seqera's documented figure for its own run of that release is $34.90 with Fusion. That run is not the megatest run, so the comparison is a bracket, not a reconciliation.

## The runs with no usage data

The June 2025 megatests (3.19.0, Nextflow 25.04.2) record `-` for `%cpu` and peak RSS on every task. A tool that read `-` as zero would report 100% waste on those runs and advise shrinking every process to the minimum. nf-audit treats missing usage as unknown:
- cost is still computed from requests;
- efficiency and right-sizing are reported as n/a.

Those runs are excluded from every efficiency figure above.

## Doing something about it

Nextflow lets you override any process's resources from a config file, without touching the pipeline. `nf-audit analyze --config-out` writes one from observed peaks plus a 25% margin for CPU and memory, and 1.5× the longest run time for `time`. It writes only values it lowers, and it leaves out any process whose tasks needed more than their first attempt's request, were retried at a larger one, or were killed, since a fixed value would take away the pipeline's retry escalation. I checked the output in Nextflow itself: it loads with `-c` and the lowered requests take effect. For `QUALIMAP_RNASEQ` it produces the first block below. The RSEM block matches the fix now proposed upstream (see the end of this section):

```groovy
// from rnaseq 3.15.1 star_salmon
withName: 'NFCORE_RNASEQ:RNASEQ:QUALIMAP_RNASEQ' {
    cpus   = 2
    memory = '7.GB'
    time   = '3.h'
}
// rnaseq star_rsem: the memory and time proposed upstream in nf-core/rnaseq#1959
withName: '.*:QUANTIFY_RSEM:RSEM_CALCULATEEXPRESSION' {
    memory = '16.GB'
    time   = '24.h'
}
```

Upstream multiplies both values by `task.attempt`, so a retry still gets more; the fixed values above don't. The RSEM block is not nf-audit's raw output. From 3.22.0 alone, nf-audit proposes 2 CPUs / 12 GiB and leaves `time` at **16 h**, because it only ever shrinks a request. The later releases show that 16 h is exactly the limit the task runs into. The block sets:
- the memory from the 9.45 GiB peak;
- the time from the longest completed attempt (22.6 h; 24 h is only ~6% above it).

**CPUs are left at 12**, as upstream chose, because the CPU data is noisy. The completed tasks' median is about one core, but one 3.27.0 task averaged 5.1 cores and one task in an off-release run 14.2. If you want to go further on your own infrastructure, the thread benchmark above is the evidence: 4 threads reached 68% of the 12-thread speed locally. Validate on your own runs first.

Raising a time limit when observed run times approach it is now on nf-audit's roadmap.

What the fragments would save, as estimates:

| run | allocated cost | estimated saving |
|---|---:|---:|
| 3.15.1 `star_salmon` | $79.23 | $41.42 (52%) for nf-audit's full 52-process fragment (the `QUALIMAP_RNASEQ` block alone: $13.50), priced at the same run times |
| 3.22.0 `star_rsem`, RSEM line only, 12 CPU / 16 GiB as above | $221.80 | $103.51 (47%) at the same run times |
| the same with CPUs also cut to 4 | $221.80 | $162.65 at the same run times, or $134.57 if the task slows by the 1.47× measured locally from 12 to 4 threads |

These are estimates from one run each, and the fragments are sized to *those* samples. Validate on one of your own runs before rolling out: a process that is already CPU-bound will slow down if you cut its cores. For scale, at the list rate a task-hour costs $3.00 at 12 CPU / 72 GiB, $1.60 at 12 / 16 and $0.80 at 4 / 16.

**Upstream.** I reported the `star_rsem` timeouts to nf-core with the data above: [nf-core/rnaseq#1957](https://github.com/nf-core/rnaseq/issues/1957). A maintainer confirmed the numbers and opened [#1959](https://github.com/nf-core/rnaseq/pull/1959) against the `dev` branch. It is open and awaiting review at the time of writing (2 October 2026), and it:
- lowers the RSEM step's memory from 72 GB to 16 GB;
- raises its time limit from 16 h to 24 h;
- drops a transcript BAM that RSEM writes by default and the pipeline never uses;
- leaves CPUs at 12.

The first full-size `star_rsem` test run after the fix is released will show whether it holds; I'll post those numbers on the issue.

## Caveats

- Costs are at list-style rates on requested resources. They are not an AWS invoice.
- One run per release and route, picked by the rule stated at the top, `test_full` samples only: one observation per release, not a distribution.
- Cross-release cost changes mix pipeline and infrastructure changes (see section 1).
- The 3.19.0 runs carry no usage data and are excluded from efficiency figures.

## Try it

```bash
nf-audit inspect results/pipeline_info/execution_report_*.html
nf-audit analyze --report results/pipeline_info/execution_report_*.html --config-out right_sized.config
```

MIT-licensed, one static binary: [github.com/OtoYuki/nf-audit](https://github.com/OtoYuki/nf-audit). If you run Nextflow at scale and want this done on your own runs, I do that as a fixed-fee audit.
