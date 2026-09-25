# Examples

Output of nf-audit on public nf-core/rnaseq AWS megatest runs (`s3://nf-core-awsmegatests`, `-profile test_full`, 8 ENCODE samples, ~123 GB of FASTQ). Generated 2026-09-19 with the `seqera-compute` preset ($0.10/CPU-h + $0.025/GiB-h). Inputs are not in the repo; `scripts/pull-megatests.sh rnaseq <prefix>` fetches them, and each `*.files` list names the exact report behind each column.

| file | what |
|---|---|
| `rnaseq-3.15.1-star_salmon.md` | one run, the release Seqera's own $34.90 / $58.40 figure is for |
| `rnaseq-3.15.1-star_salmon.nf-audit.config` | the right-sizing fragment for it |
| `rnaseq-3.22.0-star_rsem.md` | one run on the RSEM branch; `RSEM_CALCULATEEXPRESSION` at 9% CPU and 11% memory efficiency |
| `rnaseq-3.22.0-star_rsem.nf-audit.config` | its fragment; the RSEM line alone is $185 of a $294 run |
| `rnaseq-star_salmon-releases.md` | 25 releases, 3.1 (May 2021) → 3.26.0 (May 2026), `star_salmon` branch |
| `rnaseq-star_rsem-releases.md` | 20 releases on the `star_rsem` branch (newer releases only ran it under `results-dev`) |

Commands:

```
nf-audit analyze --trace .../execution_trace_2024-09-16_16-33-21.txt \
                 --report .../execution_report_2024-09-16_16-33-21.html \
                 --top 20 --config-out rnaseq-3.15.1-star_salmon.nf-audit.config > rnaseq-3.15.1-star_salmon.md
nf-audit compare --top 12 $(cat rnaseq-star_salmon-releases.files) > rnaseq-star_salmon-releases.md
```

Three things the tables show, all verified against the raw trace rows:

1. **Unused allocation has sat at 62–68% of the bill for five years on `star_salmon`**, while the bill itself fell from $138 (3.1) to $84 (3.11, March 2023, wall time 10 h → 4 h) and $60 (3.26.0). The pipeline got cheaper; the requests never got closer to what the tools use.
2. **`QUALIMAP_RNASEQ` was the second most expensive step through 3.18 (22–23% of cost at 16% CPU and 14% memory efficiency: 6 CPUs and 36 GB for a mostly single-threaded tool) and dropped to 9% in 3.22.0** when its run time fell from 11.6 h to 4.6 h summed. `STAR_ALIGN` did not get dearer; its share rose to 31% because everything around it shrank.
3. **The `star_rsem` branch regressed.** `RSEM_CALCULATEEXPRESSION` per sample went from 2–2.7 cores, 30 GB, 3.7–6.7 h in 3.17.0 to about 1 core, 7–9 GB, 7.7–11 h in 3.22.0 while still requesting 12 CPUs and 72 GB for 16 h. That one process is 75% of a $294 run at 9% CPU efficiency; the branch costs 3.8× the `star_salmon` branch on the same samples, and 81–84% of what it is billed for goes unused. Right-sizing it to 2 CPUs / 12 GB is the single largest saving in this corpus. Two changes sit behind this. From 3.21.0 (nf-core/rnaseq PR #1604) STAR runs as its own task and RSEM only quantifies the transcriptome BAM, but the task keeps the module's `process_high` request; that is why peak memory fell from 30 GB to under 10 GB. Separately, RSEM's share already jumped from 39–45% to 64–68% at 3.11.1 with identical code, container and arguments, coinciding with the move of these tests to new infrastructure in 3.11.0 (#981); that cause is not established.

Caveats that belong next to any of these numbers: the megatest runs are Seqera's Tower-forged AWS Batch compute environment, priced here at Seqera Compute's list rate rather than what AWS billed (see the README section on the Platform estimate); the June 2025 runs (3.19.0, Nextflow 25.04.2) carry no usage metrics and are excluded from every efficiency figure; one run per release is a single observation, not a distribution.
