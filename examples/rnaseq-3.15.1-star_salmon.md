# nf-audit report

- run: nf-core/rnaseq 3.15.1 · test_full_aws · Nextflow 24.04.4 · Fusion 2.3 · wall 5h 8m 46s
- trace: `data/rnaseq/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_salmon/pipeline_info/execution_trace_2024-09-16_16-33-21.txt`
- report: `data/rnaseq/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_salmon/pipeline_info/execution_report_2024-09-16_16-33-21.html`
- rates: `seqera-compute` ($0.1000/CPU-h, $0.0250/GiB-h)
- tasks: 283 across 52 processes

## Totals

| metric | value |
|---|---|
| billed cost | $79.23 |
| of which allocated but unused | $52.08 (66% of the $79.23 metered) |
| of which spent on failed attempts | $0.00 |
| task run time (sum) | 50.8 h |
| CPU-hours requested / used | 316.9 / 130.0 (41%) |
| GiB-hours requested / used | 1901.4 / 568.1 (30%) |

## Cost by process (top 20)

| process | tasks | run time | cost | share | cpu eff | mem eff | waste | retries | failed |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| STAR_ALIGN_IGENOMES | 8 | 6.4h | $19.28 | 24% | 47% | 50% | $9.88 | 0 | 0 |
| QUALIMAP_RNASEQ | 8 | 12.0h | $18.00 | 23% | 16% | 14% | $15.37 | 0 | 0 |
| TRIMGALORE | 8 | 4.5h | $13.49 | 17% | 61% | 8% | $9.49 | 0 | 0 |
| PICARD_MARKDUPLICATES | 8 | 5.3h | $7.99 | 10% | 22% | 76% | $3.63 | 0 | 0 |
| QUANTIFY_PSEUDO_ALIGNMENT:SALMON_QUANT | 8 | 2.1h | $3.19 | 4% | 99% | 48% | $1.01 | 0 | 0 |
| RSEQC_READDUPLICATION | 8 | 1.6h | $2.46 | 3% | 17% | 54% | $1.49 | 0 | 0 |
| DUPRADAR | 8 | 7.4h | $1.84 | 2% | 72% | 9% | $1.21 | 0 | 0 |
| FASTQC | 8 | 1.2h | $1.79 | 2% | 33% | 13% | $1.41 | 0 | 0 |
| STRINGTIE_STRINGTIE | 8 | 59m | $1.47 | 2% | 20% | 3% | $1.32 | 0 | 0 |
| RSEQC_READDISTRIBUTION | 8 | 57m | $1.42 | 2% | 17% | 3% | $1.30 | 0 | 0 |
| RSEQC_JUNCTIONSATURATION | 8 | 48m | $1.21 | 2% | 17% | 5% | $1.09 | 0 | 0 |
| QUANTIFY_STAR_SALMON:SALMON_QUANT | 8 | 44m | $1.09 | 1% | 91% | 16% | $0.59 | 0 | 0 |
| RSEQC_BAMSTAT | 8 | 42m | $1.05 | 1% | 17% | 0% | $0.97 | 0 | 0 |
| RSEQC_JUNCTIONANNOTATION | 8 | 37m | $0.93 | 1% | 18% | 0% | $0.86 | 0 | 0 |
| SAMTOOLS_SORT | 8 | 36m | $0.90 | 1% | 94% | 15% | $0.48 | 0 | 0 |
| SUBREAD_FEATURECOUNTS | 8 | 32m | $0.80 | 1% | 50% | 2% | $0.63 | 0 | 0 |
| SALMON_INDEX | 1 | 29m | $0.72 | 1% | 69% | 51% | $0.31 | 0 | 0 |
| BEDTOOLS_GENOMECOV_REV | 8 | 55m | $0.23 | 0% | 102% | 54% | $0.06 | 0 | 0 |
| BEDTOOLS_GENOMECOV_FW | 8 | 54m | $0.23 | 0% | 103% | 54% | $0.06 | 0 | 0 |
| RSEQC_INNERDISTANCE | 8 | 7m | $0.18 | 0% | 21% | 2% | $0.16 | 0 | 0 |

## Right-sizing (margin applied to observed peaks): est. saving $41.42 (52%)

| process | cpus now → new | memory now → new | time now → new | est. saving |
|---|---:|---:|---:|---:|
| QUALIMAP_RNASEQ | 6 → 2 | 36 GB → 7 GB | 8h → 3h | $13.50 |
| TRIMGALORE | 12 → 10 | 72 GB → 8 GB | 16h → 1h | $8.09 |
| STAR_ALIGN_IGENOMES | 12 → 8 | 72 GB → 46 GB | 16h → 2h | $6.75 |
| PICARD_MARKDUPLICATES | 6 → 2 | 36 GB → 36 GB | 8h → 2h | $2.13 |
| STRINGTIE_STRINGTIE | 6 → 2 | 36 GB → 2 GB | 8h → 30m | $1.23 |
| RSEQC_READDISTRIBUTION | 6 → 2 | 36 GB → 2 GB | 8h → 30m | $1.19 |
| FASTQC | 6 → 3 | 36 GB → 10 GB | 8h → 15m | $1.13 |
| RSEQC_JUNCTIONSATURATION | 6 → 2 | 36 GB → 4 GB | 8h → 15m | $0.96 |
| DUPRADAR | 1 → 1 | 6 GB → 1 GB | 20h → 3h | $0.92 |
| RSEQC_BAMSTAT | 6 → 2 | 36 GB → 1 GB | 8h → 15m | $0.89 |
| RSEQC_JUNCTIONANNOTATION | 6 → 2 | 36 GB → 1 GB | 8h → 15m | $0.79 |
| QUANTIFY_PSEUDO_ALIGNMENT:SALMON_QUANT | 6 → 6 | 36 GB → 22 GB | 8h → 30m | $0.75 |
| RSEQC_READDUPLICATION | 6 → 2 | 36 GB → 36 GB | 8h → 30m | $0.66 |
| QUANTIFY_STAR_SALMON:SALMON_QUANT | 6 → 6 | 36 GB → 8 GB | 8h → 15m | $0.51 |
| SUBREAD_FEATURECOUNTS | 6 → 5 | 36 GB → 2 GB | 8h → 15m | $0.50 |
| SAMTOOLS_SORT | 6 → 6 | 36 GB → 7 GB | 8h → 15m | $0.44 |
| RSEQC_INNERDISTANCE | 6 → 2 | 36 GB → 2 GB | 8h → 15m | $0.15 |
| SALMON_INDEX | 6 → 6 | 36 GB → 24 GB | 8h → 45m | $0.15 |
| BEDGRAPH_BEDCLIP_BEDGRAPHTOBIGWIG_REVERSE:UCSC_BEDCLIP | 6 → 1 | 36 GB → 1 GB | 8h → 15m | $0.12 |
| BEDGRAPH_BEDCLIP_BEDGRAPHTOBIGWIG_FORWARD:UCSC_BEDCLIP | 6 → 1 | 36 GB → 1 GB | 8h → 15m | $0.11 |

Savings assume the same run time at the smaller allocation, which holds for memory and for CPU-bound processes that were not using the extra cores. Validate on one real run before rolling out; a process at 100% CPU efficiency will slow down if you cut its cores.
