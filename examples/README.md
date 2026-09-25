# Examples

Output of nf-audit on public nf-core/rnaseq AWS megatest runs (`s3://nf-core-awsmegatests`). These are full-size test runs: 8 paired-end human RNA-seq samples from the GM12878, K562, MCF7 and H1 cell lines, ~123.5 GB of FASTQ.

- **Rates:** the `seqera-compute` preset ($0.10/CPU-h + $0.025/GiB-h).
- **When:** first generated 2026-09-19; `star_rsem` tables updated 2026-09-25.
- **Inputs:** not in the repo. `scripts/pull-megatests.sh rnaseq <prefix>` fetches them, and each `*.files` list names the exact report behind each column.
- **Units:** memory is GiB; Nextflow labels it "GB".

| file | what |
|---|---|
| `rnaseq-3.15.1-star_salmon.md` | one run of the release Seqera's own $34.90 / $58.40 figure is for |
| `rnaseq-3.15.1-star_salmon.nf-audit.config` | the right-sizing fragment for it |
| `rnaseq-3.22.0-star_rsem.md` | one run on the RSEM branch; `RSEM_CALCULATEEXPRESSION` at 9% CPU and 11% memory efficiency |
| `rnaseq-3.22.0-star_rsem.nf-audit.config` | its fragment. The RSEM line alone (2 CPU / 12 GiB) is an estimated $185 saving on a $294 run, priced at the same run times |
| `rnaseq-star_salmon-releases.md` | 25 releases, 3.1 (May 2021) → 3.26.0 (May 2026), `star_salmon` branch |
| `rnaseq-star_rsem-releases.md` | 25 releases on the `star_rsem` branch, 3.1 → 3.27.0; details below |
| `rsem-threads/` | the 3.22.0 RSEM command run locally at 1–12 threads on the megatest's own chr1 reads: scaling, a parse-only timing, output hashes, I/O |

Notes on `rnaseq-star_rsem-releases.md`:
- 3.22.1, 3.23.0, 3.24.0 and 3.25.0 are runs that failed: `RSEM_CALCULATEEXPRESSION` hit the 16 h limit.
- 3.26.0 is left out because both of its runs stopped early for reasons unrelated to RSEM.
- The 3.15.1 report is a `-resume` run: 195 of 271 tasks are cached and priced at their recorded run times.

Commands:

```
nf-audit analyze --trace .../execution_trace_2024-09-16_16-33-21.txt \
                 --report .../execution_report_2024-09-16_16-33-21.html \
                 --top 20 --config-out rnaseq-3.15.1-star_salmon.nf-audit.config > rnaseq-3.15.1-star_salmon.md
nf-audit compare --top 12 $(cat rnaseq-star_salmon-releases.files) > rnaseq-star_salmon-releases.md
```

What the tables show:

1. **Unused allocation has stayed at 62–68% of allocated cost for five years on `star_salmon`.** Over the same period the allocated cost fell from $138 (3.1) to $84 (3.11.1) and $60 (3.26.0). Two of those steps are not about tools getting faster:
   - **3.10.1 → 3.11.1:** nf-core changed how these tests are launched in 3.11.0 (#981: new CI workflow and compute environment; profile `test_full,aws_tower` → `test_full_aws`).
   - **3.25.0 → 3.26.0:** nf-core changed two processes itself. `TRIMGALORE` went from 12 CPU / 72 GiB to 8 CPU / 1 GiB (#1836, #1841, #1842), and the legacy STAR 2.6.1d pin was dropped (#1835). These account for $19.51 of the $19.65 drop.
2. **`QUALIMAP_RNASEQ` requests 6 CPUs / 36 GiB in every release, and no task of it averaged more than 1.08 cores.**
   - 3.1–3.8.1: third most expensive process.
   - 3.9–3.18: second, at 18–23% of cost and 15–17% CPU efficiency.
   - 3.22.0: 9%, when its summed run time fell from 11.6 h to 4.6 h.
   - `STAR_ALIGN_IGENOMES` went the other way: from $19.24 (3.18.0) to $24.43 (3.22.0), 31% of that run.
3. **On the `star_rsem` branch, `RSEM_CALCULATEEXPRESSION` is over-requested and, on the test infrastructure, sometimes too slow for its time limit.**
   - **Since STAR moved out of the task** (3.21.0, nf-core/rnaseq #1604), RSEM tasks request 12 CPUs / 72 GiB / 16 h. Across the 45 completed tasks from 3.22.0 onward they average one to two cores (median 104%, mean 120%) and peak at or below 9.45 GiB. In 3.22.0 that one process is 75% of a $294 run at 9% CPU efficiency.
   - **The full-size test failed in 3.22.1, 3.23.0, 3.24.0 and 3.25.0** because an RSEM task hit the 16 h limit (`Job attempt duration exceeded timeout`, per each run's report). In three of the four, the task's last logged line is `rsem-parse-alignments` progress (`Parsed 245000000`, `255000000` and `73000000 entries`). That is RSEM's single-threaded first step.
   - **In 3.27.0** the test passed. One task needed three attempts (exit 175, Fusion Snapshots errors), and two tasks ran past the 16 h limit without being stopped (22.6 h and 19.4 h). RSEM is 89% of that $464 run.
   - **Locally the same command does use its threads**: 4.36× faster at 12 threads than at 1. The parse step alone takes 93–98 s for 18 M entries (see `rsem-threads/`). Why parsing is so much slower on the test infrastructure is not established.
   - **Earlier, separately:** RSEM's share jumped from 39–45% to 64–68% at 3.11.1, with identical code, container and arguments, at the same time as the test-launch change in 3.11.0 (#981). That cause is not established either.

Caveats that belong next to any of these numbers:
- The runs are priced at Seqera Compute's list rate, not at what AWS billed (see the README section on the Platform estimate).
- The June 2025 runs (3.19.0, Nextflow 25.04.2) carry no usage metrics and are excluded from every efficiency figure.
- There are one or two runs per release: a single observation, not a distribution.
