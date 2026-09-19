# State on Friday Sept 19, and what Monday looks like

## Done on Sept 19 (evening)

1. `cargo build --release && cargo test`: 14 tests, clippy and fmt clean.
2. Megatest data pulled: rnaseq 3.19.0 (`results-0bb032c1…`), 3.15.1 (`results-4053b2ec…`), and one run per tagged release 3.1 → 3.26.0 (25 prefixes) under `data/rnaseq/`. `scripts/pull-megatests.sh` was rewritten: the original only handled the newest bucket layout and would have failed on every release before 3.19.
3. `nf-audit inspect` on the first real report failed twice, both fixed with tests:
   - `window.data` string search hit `window.data_byprocess = {}` in the report's helper script → "no `trace` array". Now matches the assignment specifically.
   - The blob is not valid JSON: every task's `script` field is JavaScript-escaped (`\'`). Now rewritten before parsing.
4. A third problem the plan did not anticipate: the 3.19.0 run has **no usage metrics** (`%cpu`, `peak_rss` all `-`, for every task, trace and report). The analysis treated missing as zero and printed "100% waste, shrink STAR to 1 CPU / 1 GB". Now "unknown" is a first-class state: cost is still exact, efficiency/waste/right-sizing say n/a, and `inspect` reports metered-task counts. The June 2025 megatests (Nextflow 25.04.2, Fusion 2.4) are all unmetered; 3.1–3.18 and 3.22+ are fully metered.
5. Reconciliation done, against the right anchor. Seqera's $34.90 / $58.40 is for **3.15.1**, from their own run, not the 3.19.0 megatest (README corrected). Megatest 3.15.1 `star_salmon`: 283 tasks, 5h 8m wall, 316.9 CPU-h (matches the header figure Nextflow prints), `seqera-compute` $79.23, `aws-m5-ondemand` $17.75, `aws-m5-spot` $6.21. Platform's formula is dominant-resource × instance price × (complete − start); ours is additive at a flat rate on realtime. The gap paragraph is in the README under "How this differs from Seqera Platform's estimate".
6. `nf-audit compare` added: many reports in, per-run table plus a process × run cost-share matrix. Run on 17 star_salmon releases (3.1 → 3.17): allocated cost fell 45% between 3.10 and 3.11 and has been flat since; **unused allocation has sat at 63–68% for five years**; QUALIMAP_RNASEQ climbed from 17% to 23% of the bill while STAR sits at 23–24%.
7. Report header parsed (`RunMeta`): revision, profile, Nextflow version, Fusion, wall duration, Nextflow's own CPU-hours; shown by `inspect` and at the top of every report.

8. Published: https://github.com/OtoYuki/nf-audit (public, MIT, topics set), release v0.1.0 with a Linux x86_64 tarball. Board B1–B3 ticked. B4 still needs, by hand: pin the repo on the GitHub profile (no API for it) and the PR to sharkLoc/rust-in-bioinformatics.

## Monday Sept 22

1. Pin `nf-audit` on github.com/OtoYuki; open the sharkLoc/rust-in-bioinformatics PR; tick B4.
2. Re-run the cross-version table with the newest releases included and pin the exact command in the blog draft:
   `./target/release/nf-audit compare --top 12 $(cat /path/to/compare-salmon.txt) > releases-salmon.md`
   (the file list is written by the Sept 19 session; regenerate from `data/rnaseq/*/aligner_star_salmon/pipeline_info/execution_report_*.html` and drop reruns/failed runs by hand).
3. Same for `star_rsem` (RSEM_CALCULATEEXPRESSION dominates those; different story, worth one paragraph).
4. sarek `results-dev`: pull, inspect, one analyze. Only if it is metered.
5. Blog post draft "Where nf-core/rnaseq's $35 goes, process by process": the 3.15.1 analyze report, the compare matrix as the chart, the Platform-vs-nf-audit paragraph, the unmetered-runs caveat as the honesty section.
6. RustQC issue #141 (HashMap iteration order) or #129 (hts-sys pin) as the PR of the week.
