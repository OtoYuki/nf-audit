# Examples

Output of nf-audit on public nf-core/rnaseq AWS megatest runs (`s3://nf-core-awsmegatests`). These are full-size test runs: 8 paired-end human RNA-seq samples from the GM12878, K562, MCF7 and H1 cell lines, ~123.5 GB of FASTQ.

- **Rates:** the `seqera-compute` preset ($0.10/CPU-h + $0.025/GiB-h).
- **When:** first generated 2026-09-19; `star_rsem` tables updated 2026-09-25.
- **Inputs:** not in the repo. `scripts/pull-megatests.sh rnaseq <prefix>` fetches them, and each `*.files` list names the exact report behind each column.
- **Which runs:** for each release tag and route, the last successful run stored under that tag's own `results-<commit>/` prefix. On `star_rsem`, where a release has none, the last run in which `RSEM_CALCULATEEXPRESSION` failed.
- **Left out:**
  - The two unmetered 3.19.0 runs.
  - Releases where no run qualifies (3.11.0, 3.13.0 `star_salmon`, 3.26.0 `star_rsem`).
  - Releases with no report in the bucket (3.0, 3.10, 3.20.0, 3.21.0, and 3.27.0 on `star_salmon`).
  - Runs under other prefixes: a 2023-11-15 re-run of 3.12.0's commit, the release/3.25.0 merge commit, and `results-dev`.
- **Units:** memory is GiB; Nextflow labels it "GB".

| file | what |
|---|---|
| `rnaseq-3.15.1-star_salmon.md` | one run of the release Seqera's own $34.90 / $58.40 figure is for |
| `rnaseq-3.15.1-star_salmon.nf-audit.config` | the right-sizing fragment for it |
| `rnaseq-3.22.0-star_rsem.md` | one run on the RSEM branch; `RSEM_CALCULATEEXPRESSION` at 9% CPU and 11% memory efficiency |
| `rnaseq-3.22.0-star_rsem.nf-audit.config` | its fragment. The RSEM line alone (2 CPU / 12 GiB) is an estimated $185 saving on a $294 run, priced at the same run times. It keeps the 16 h limit that later runs fail on |
| `rnaseq-star_salmon-releases.md` | 30 releases, 3.1 (May 2021) → 3.26.0 (May 2026), `star_salmon` branch |
| `rnaseq-star_rsem-releases.md` | 31 releases on the `star_rsem` branch, 3.1 → 3.27.0; details below |
| `rsem-threads/` | the 3.22.0 RSEM command run locally at 1–12 threads on the megatest's own chr1 reads: scaling, a parse-only timing, output hashes, I/O |

Notes on `rnaseq-star_rsem-releases.md`:
- 3.22.1, 3.23.0, 3.24.0 and 3.25.0 are runs that failed: `RSEM_CALCULATEEXPRESSION` hit the 16 h limit.
- 3.26.0 is left out because both of its runs stopped early for reasons unrelated to RSEM.
- The 3.15.1 report is a `-resume` run: 195 of 271 tasks are cached and priced at their recorded run times. 3.5's successful runs on both routes are also `-resume` runs.
- Reports from 3.13.x show the revision as `master`.

Commands:

```
nf-audit analyze --trace .../execution_trace_2024-09-16_16-33-21.txt \
                 --report .../execution_report_2024-09-16_16-33-21.html \
                 --top 20 --config-out rnaseq-3.15.1-star_salmon.nf-audit.config > rnaseq-3.15.1-star_salmon.md
nf-audit compare --top 12 $(cat rnaseq-star_salmon-releases.files) > rnaseq-star_salmon-releases.md
```

What the tables show:

1. **Unused allocation has stayed at 62–68% of metered allocated cost in 28 of the 30 `star_salmon` releases.** The exceptions are 3.5 (81%, a `-resume` run) and 3.6 (73%, 13 failed attempts). Over the same period the allocated cost fell from $139 (3.1) to $84 (3.11.1) and $60 (3.26.0). Changes between releases mix pipeline and infrastructure:
   - **3.10.1 → 3.11.1:** nf-core changed how these tests are launched in 3.11.0 (#981: new CI workflow with a renamed compute-environment secret; profile `test_full,aws_tower` → `test_full_aws`).
   - **Between 3.18.0 and 3.22.0:** the profile changed again (`test_full_aws` → `test_full`, already so in the 3.19.0 runs), with Nextflow 24.10 → 25.04.
   - **3.25.0 → 3.26.0:** nf-core upgraded two processes and resized one of them.
     - `TRIMGALORE` moved to Trim Galore 2.1.0 (#1789) and from 12 CPU / 72 GiB to 8 CPU / 1 GiB (#1836, #1841, #1842). Its run time fell from 4.38 h to 1.62 h, and its cost from $13.15 to $1.34.
     - The legacy STAR 2.6.1d pin was dropped for 2.7.11b (#1835); STAR went from $23.86 to $16.16.
     - The two differences sum to $19.51 of the $19.65 net drop. Other processes moved by up to about ±$3, so the split is approximate. TRIMGALORE's own split between run time and request depends on the order you apply them.
2. **`QUALIMAP_RNASEQ` requests 6 CPUs / 36 GiB in every release, and no task of it averaged more than 1.08 cores.** In 3.6, two tasks that failed their first attempt were retried at 12 / 72.
   - 3.1–3.4 and 3.7–3.8.1: third most expensive process; 3.5: fourth.
   - 3.6 and 3.9–3.18: second, at 18–23% of cost and 13–17% CPU efficiency.
   - 3.22.0: 9%, when its summed run time fell from 11.6 h to 4.6 h.
   - `STAR_ALIGN_IGENOMES` went the other way across the 3.22.0 profile change: from $19.24 (3.18.0) to $24.43 (3.22.0), 31% of that run.
3. **On the `star_rsem` branch, `RSEM_CALCULATEEXPRESSION` is over-requested and, on the test infrastructure, sometimes too slow for its time limit.**
   - **Since STAR moved out of the task** (3.21.0, nf-core/rnaseq #1604), RSEM tasks request 12 CPUs / 72 GiB / 16 h. Across the 45 completed tasks from 3.22.0 onward they average a median of 1.04 cores (mean 1.20, range 0.25–5.1) and peak at or below 9.45 GiB. In 3.22.0 that one process is 75% of a $294 run at 9% CPU efficiency.
   - **The full-size test failed in 3.22.1, 3.23.0, 3.24.0 and 3.25.0** because an RSEM task hit the 16 h limit (`Job attempt duration exceeded timeout`, per each run's report). In three of the four, the task's last logged line is `rsem-parse-alignments` progress (`Parsed 245000000`, `255000000` and `73000000 entries`). That is RSEM's single-threaded first step.
   - **In 3.27.0** the test passed. One task needed three attempts (exit 175, the code Fusion Snapshots uses for an unrecoverable error), and two tasks ran past the 16 h limit without being stopped (22.6 h and 19.4 h). RSEM is 89% of that $464 run.
   - **Locally the same command does use its threads**: 4.36× faster at 12 threads than at 1. The parse step alone takes 93–98 s for 18 M entries (see `rsem-threads/`). Why parsing is so much slower on the test infrastructure is not established.
   - **Earlier, separately:** RSEM's share jumped from 31–48% (3.1–3.10.1) to 64–68% (3.11.1–3.17.0) at the same time as the test-launch change in 3.11.0 (#981). RSEM's script, arguments and base image are identical across it; from 3.11.1 the image is served through Wave, and Nextflow went from 22.10.4 to 23.03.0-edge. That cause is not established either.
   - **`dev`-branch runs** (`results-dev`, not in the tables): of 19 `star_rsem` runs, 7 stopped on the same RSEM 16 h timeout. In six, stdout ends on `Parsed N entries`. In one, it ends on `515000000 alignment lines are loaded!`, printed while `rsem-run-em` writes its output BAM after EM.

Caveats that belong next to any of these numbers:
- The runs are priced at Seqera Compute's list rate, not at what AWS billed (see the README section on the Platform estimate).
- The June 2025 runs (3.19.0, Nextflow 25.04.2) carry no usage metrics and are excluded from every efficiency figure.
- There is one run per release and route: a single observation, not a distribution.
