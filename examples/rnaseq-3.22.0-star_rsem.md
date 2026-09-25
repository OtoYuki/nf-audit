# nf-audit report

- run: nf-core/rnaseq 3.22.0 · test_full · Nextflow 25.04.8 · Fusion 2.4 · wall 12h 57m 8s
- trace: `data/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info/execution_trace_2025-11-27_10-50-37.txt`
- report: `data/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info/execution_report_2025-11-27_10-50-37.html`
- rates: `seqera-compute` ($0.1000/CPU-h, $0.0250/GiB-h)
- tasks: 301 across 49 processes

## Totals

| metric | value |
|---|---|
| billed cost | $294.25 |
| of which allocated but unused | $247.22 (84% of the $294.25 metered) |
| of which spent on failed attempts | $0.00 |
| task run time (sum) | 126.0 h |
| CPU-hours requested / used | 1177.0 / 192.4 (16%) |
| GiB-hours requested / used | 7062.0 / 1113.2 (16%) |

## Cost by process (top 20)

| process | tasks | run time | cost | share | cpu eff | mem eff | waste | retries | failed |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| RSEM_CALCULATEEXPRESSION | 8 | 73.9h | $221.80 | 75% | 9% | 11% | $199.16 | 0 | 0 |
| STAR_ALIGN_IGENOMES | 8 | 6.5h | $19.38 | 7% | 32% | 45% | $11.72 | 0 | 0 |
| TRIMGALORE | 8 | 4.6h | $13.68 | 5% | 58% | 2% | $10.33 | 0 | 0 |
| PICARD_MARKDUPLICATES | 8 | 5.8h | $8.63 | 3% | 22% | 77% | $3.90 | 0 | 0 |
| QUALIMAP_RNASEQ | 8 | 4.6h | $6.84 | 2% | 17% | 15% | $5.75 | 0 | 0 |
| SALMON_QUANT | 8 | 2.5h | $3.70 | 1% | 98% | 48% | $1.19 | 0 | 0 |
| DUPRADAR | 8 | 12.2h | $3.05 | 1% | 47% | 8% | $2.32 | 0 | 0 |
| RSEQC_READDUPLICATION | 8 | 1.6h | $2.47 | 1% | 17% | 55% | $1.49 | 0 | 0 |
| FASTQC | 8 | 1.4h | $2.04 | 1% | 32% | 12% | $1.64 | 0 | 0 |
| STRINGTIE_STRINGTIE | 8 | 60m | $1.50 | 1% | 20% | 3% | $1.35 | 0 | 0 |
| RSEQC_READDISTRIBUTION | 8 | 59m | $1.49 | 1% | 17% | 3% | $1.36 | 0 | 0 |
| SAMTOOLS_SORT_QUALIMAP | 8 | 56m | $1.41 | 0% | 88% | 15% | $0.79 | 0 | 0 |
| RSEQC_JUNCTIONSATURATION | 8 | 52m | $1.30 | 0% | 18% | 5% | $1.17 | 0 | 0 |
| RSEQC_BAMSTAT | 8 | 45m | $1.12 | 0% | 18% | 0% | $1.04 | 0 | 0 |
| RSEQC_JUNCTIONANNOTATION | 8 | 40m | $1.00 | 0% | 18% | 0% | $0.93 | 0 | 0 |
| SAMTOOLS_SORT | 8 | 37m | $0.92 | 0% | 93% | 15% | $0.50 | 0 | 0 |
| SALMON_INDEX | 1 | 27m | $0.69 | 0% | 73% | 51% | $0.28 | 0 | 0 |
| FQ_LINT_AFTER_TRIMMING | 8 | 58m | $0.48 | 0% | 50% | 5% | $0.37 | 0 | 0 |
| FQ_LINT | 8 | 57m | $0.47 | 0% | 47% | 5% | $0.37 | 0 | 0 |
| BEDTOOLS_GENOMECOV_FW | 8 | 1.1h | $0.27 | 0% | 96% | 55% | $0.08 | 0 | 0 |

## Cost by task tag (top 20)

The tag is the text in parentheses after the process name. nf-core pipelines usually tag with the sample ID, which makes this a per-sample cost; other tags (lanes, intervals, reference files) show up as their own rows. Tagged tasks carry 100.0% of the cost.

| tag | tasks | processes | run time | cost | share | waste |
|---|---:|---:|---:|---:|---:|---:|
| MCF7_REP2 | 36 | 36 | 17.8h | $42.45 | 14% | $35.60 |
| H1_REP1 | 36 | 36 | 17.8h | $40.87 | 14% | $34.20 |
| MCF7_REP1 | 36 | 36 | 17.2h | $39.99 | 14% | $33.70 |
| K562_REP2 | 36 | 36 | 16.1h | $38.64 | 13% | $32.39 |
| H1_REP2 | 36 | 36 | 14.7h | $35.18 | 12% | $29.64 |
| GM12878_REP2 | 36 | 36 | 13.8h | $32.65 | 11% | $27.49 |
| GM12878_REP1 | 36 | 36 | 14.1h | $32.25 | 11% | $27.12 |
| K562_REP1 | 36 | 36 | 13.8h | $31.19 | 11% | $26.44 |
| genome.transcripts.fa | 1 | 1 | 27m | $0.69 | 0% | $0.28 |
| rsem/genome.fa | 2 | 2 | 5m | $0.27 | 0% | $0.26 |
| all_samples | 2 | 2 | 1m | $0.04 | 0% | $0.03 |
| (untagged) | 4 | 4 | 1m | $0.03 | 0% | $0.02 |
| 1 | 1 | 1 | 1m | $0.01 | 0% | $0.00 |
| genome.fa | 2 | 2 | 1m | $0.01 | 0% | $0.00 |
| null | 1 | 1 | 1m | $0.00 | 0% | $0.00 |

## Right-sizing (margin applied to observed peaks): est. saving $222.61 (76%)

| process | cpus now → new | memory now → new | time now → new | est. saving |
|---|---:|---:|---:|---:|
| RSEM_CALCULATEEXPRESSION | 12 → 2 | 72 GB → 12 GB | 16h → 16h | $184.83 |
| TRIMGALORE | 12 → 10 | 72 GB → 2 GB | 16h → 2h | $8.89 |
| STAR_ALIGN_IGENOMES | 12 → 6 | 72 GB → 42 GB | 16h → 2h | $8.72 |
| QUALIMAP_RNASEQ | 6 → 2 | 36 GB → 8 GB | 8h → 1h | $5.02 |
| PICARD_MARKDUPLICATES | 6 → 2 | 36 GB → 36 GB | 8h → 2h | $2.30 |
| DUPRADAR | 1 → 1 | 6 GB → 1 GB | 20h → 4h | $1.52 |
| FASTQC | 6 → 3 | 36 GB → 8 GB | 8h → 30m | $1.36 |
| STRINGTIE_STRINGTIE | 6 → 2 | 36 GB → 2 GB | 8h → 15m | $1.25 |
| RSEQC_READDISTRIBUTION | 6 → 2 | 36 GB → 2 GB | 8h → 30m | $1.24 |
| RSEQC_JUNCTIONSATURATION | 6 → 2 | 36 GB → 3 GB | 8h → 15m | $1.06 |
| RSEQC_BAMSTAT | 6 → 2 | 36 GB → 1 GB | 8h → 15m | $0.95 |
| SALMON_QUANT | 6 → 6 | 36 GB → 22 GB | 8h → 45m | $0.86 |
| RSEQC_JUNCTIONANNOTATION | 6 → 2 | 36 GB → 1 GB | 8h → 15m | $0.85 |
| SAMTOOLS_SORT_QUALIMAP | 6 → 6 | 36 GB → 7 GB | 8h → 15m | $0.68 |
| RSEQC_READDUPLICATION | 6 → 2 | 36 GB → 36 GB | 8h → 30m | $0.66 |
| SAMTOOLS_SORT | 6 → 6 | 36 GB → 7 GB | 8h → 15m | $0.44 |
| FQ_LINT_AFTER_TRIMMING | 2 → 2 | 12 GB → 1 GB | 4h → 15m | $0.27 |
| FQ_LINT | 2 → 2 | 12 GB → 1 GB | 4h → 15m | $0.26 |
| BEDGRAPH_BEDCLIP_BEDGRAPHTOBIGWIG_FORWARD:UCSC_BEDCLIP | 6 → 1 | 36 GB → 1 GB | 8h → 15m | $0.19 |
| BEDGRAPH_BEDCLIP_BEDGRAPHTOBIGWIG_REVERSE:UCSC_BEDCLIP | 6 → 1 | 36 GB → 1 GB | 8h → 15m | $0.17 |

Savings assume the same run time at the smaller allocation, which holds for memory and for CPU-bound processes that were not using the extra cores. Validate on one real run before rolling out; a process at 100% CPU efficiency will slow down if you cut its cores.
