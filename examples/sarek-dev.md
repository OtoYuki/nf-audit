# nf-audit report

- run: nf-core/sarek dev · test_full · Nextflow 25.04.6 · Fusion 2.4 · wall 5h 9m 35s
- report: `data/sarek/results-dev/pipeline_info/execution_report_2025-10-13_12-01-12.html`
- rates: `seqera-compute` ($0.1000/CPU-h, $0.0250/GiB-h)
- tasks: 226 across 85 processes

## Totals

| metric | value |
|---|---|
| billed cost | $15.26 |
| of which allocated but unused | $7.56 (50% of the $15.26 metered) |
| of which spent on failed attempts | $0.00 |
| task run time (sum) | 22.8 h |
| CPU-hours requested / used | 86.1 / 60.2 (70%) |
| GiB-hours requested / used | 266.0 / 77.9 (29%) |

## Cost by process (top 25)

| process | tasks | run time | cost | share | cpu eff | mem eff | waste | retries | failed |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| BWAMEM1_MEM | 24 | 1.0h | $3.26 | 21% | 82% | 39% | $0.92 | 0 | 0 |
| GATK4_MARKDUPLICATES | 2 | 58m | $1.30 | 9% | 25% | 78% | $0.59 | 0 | 0 |
| MUTECT2_PAIRED | 2 | 2.6h | $1.29 | 8% | 136% | 21% | $0.61 | 0 | 0 |
| BAM_VARIANT_CALLING_CNVKIT:CNVKIT_BATCH | 1 | 34m | $1.18 | 8% | 20% | 29% | $0.90 | 0 | 0 |
| BAM_VARIANT_CALLING_FREEBAYES:FREEBAYES | 2 | 3.3h | $0.83 | 5% | 99% | 15% | $0.42 | 0 | 0 |
| GATK4_APPLYBQSR | 4 | 2.0h | $0.60 | 4% | 53% | 42% | $0.30 | 0 | 0 |
| ENSEMBLVEP_VEP | 12 | 23m | $0.57 | 4% | 38% | 3% | $0.48 | 0 | 0 |
| MANTA_SOMATIC | 1 | 25m | $0.50 | 3% | 93% | 12% | $0.10 | 0 | 0 |
| MPILEUP_TUMOR:SAMTOOLS_MPILEUP | 2 | 2.0h | $0.50 | 3% | 72% | 7% | $0.33 | 0 | 0 |
| GATK4_BASERECALIBRATOR | 4 | 1.4h | $0.42 | 3% | 54% | 34% | $0.22 | 0 | 0 |
| MPILEUP_NORMAL:SAMTOOLS_MPILEUP | 2 | 1.6h | $0.40 | 3% | 71% | 7% | $0.27 | 0 | 0 |
| FREEC_SOMATIC | 1 | 42m | $0.35 | 2% | 64% | 76% | $0.10 | 0 | 0 |
| BAM_VARIANT_CALLING_FREEBAYES:FREEBAYES | 2 | 1.3h | $0.33 | 2% | 99% | 13% | $0.17 | 0 | 0 |
| STRELKA_SOMATIC | 2 | 15m | $0.30 | 2% | 77% | 15% | $0.10 | 0 | 0 |
| SNPEFF_SNPEFF | 12 | 11m | $0.29 | 2% | 25% | 8% | $0.24 | 0 | 0 |
| TIDDIT_TUMOR:TIDDIT_SV | 1 | 11m | $0.27 | 2% | 34% | 25% | $0.19 | 0 | 0 |
| BAM_VARIANT_CALLING_CNVKIT:CNVKIT_BATCH | 1 | 8m | $0.26 | 2% | 48% | 32% | $0.16 | 0 | 0 |
| MANTA_GERMLINE | 1 | 13m | $0.26 | 2% | 84% | 9% | $0.07 | 0 | 0 |
| GETPILEUPSUMMARIES_TUMOR | 2 | 30m | $0.25 | 2% | 52% | 14% | $0.18 | 0 | 0 |
| MUSE_CALL | 1 | 5m | $0.23 | 1% | 159% | 41% | $0.08 | 0 | 0 |
| BAM_VARIANT_CALLING_SINGLE_TIDDIT:TIDDIT_SV | 1 | 9m | $0.23 | 1% | 31% | 26% | $0.16 | 0 | 0 |
| STRELKA_SINGLE | 2 | 11m | $0.22 | 1% | 66% | 9% | $0.10 | 0 | 0 |
| GETPILEUPSUMMARIES_NORMAL | 2 | 26m | $0.22 | 1% | 53% | 18% | $0.15 | 0 | 0 |
| TIDDIT_NORMAL:TIDDIT_SV | 1 | 7m | $0.18 | 1% | 34% | 26% | $0.13 | 0 | 0 |
| FASTQC | 2 | 20m | $0.17 | 1% | 49% | 24% | $0.10 | 0 | 0 |

## Cost by task tag (top 25)

The tag is the text in parentheses after the process name. nf-core pipelines usually tag with the sample ID, which makes this a per-sample cost; other tags (lanes, intervals, reference files) show up as their own rows. Tagged tasks carry 99.9% of the cost.

| tag | tasks | processes | run time | cost | share | waste |
|---|---:|---:|---:|---:|---:|---:|
| HCC1395T_vs_HCC1395N | 105 | 43 | 12.5h | $6.87 | 45% | $3.68 |
| HCC1395N | 75 | 34 | 5.7h | $4.55 | 30% | $2.06 |
| HCC1395T | 29 | 15 | 4.1h | $3.52 | 23% | $1.41 |
| HCC1395T-1 | 2 | 2 | 14m | $0.15 | 1% | $0.06 |
| HCC1395N-1 | 2 | 2 | 12m | $0.14 | 1% | $0.05 |
| (untagged) | 1 | 1 | 1m | $0.01 | 0% | $0.01 |
| Homo_sapiens_assembly38.fasta | 1 | 1 | 1m | $0.01 | 0% | $0.01 |
| dbsnp | 1 | 1 | 1m | $0.00 | 0% | $0.00 |
| intervals | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| GC_G1000_hg38.zip | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| S07604624_Padded_Agilent_SureSelectXT_allexons_V6_UTR.bed | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| 1 | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| RT_G1000_hg38.zip | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| S07604624_Padded_Agilent_SureSelectXT_allexons_V6_UTR | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| G1000_alleles_hg38.zip | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| G1000_loci_hg38.zip | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| chr1_11981-12351 | 1 | 1 | 0m | $0.00 | 0% | $0.00 |
| chr17_38877931-38878296 | 1 | 1 | 0m | $0.00 | 0% | $0.00 |

## Right-sizing (margin applied to observed peaks): est. saving $5.01 (33%)

| process | cpus now → new | memory now → new | time now → new | est. saving |
|---|---:|---:|---:|---:|
| BAM_VARIANT_CALLING_CNVKIT:CNVKIT_BATCH | 12 → 3 | 36 GB → 14 GB | 8h → 1h | $0.81 |
| MUTECT2_PAIRED | 2 → 2 | 12 GB → 4 GB | 8h → 4h | $0.52 |
| GATK4_MARKDUPLICATES | 6 → 2 | 30 GB → 30 GB | 8h → 1h | $0.39 |
| BWAMEM1_MEM | 24 → 24 | 30 GB → 16 GB | 32h → 15m | $0.36 |
| ENSEMBLVEP_VEP | 6 → 5 | 36 GB → 3 GB | 16h → 15m | $0.35 |
| BAM_VARIANT_CALLING_FREEBAYES:FREEBAYES | 1 → 1 | 6 GB → 2 GB | 8h → 4h | $0.33 |
| MPILEUP_TUMOR:SAMTOOLS_MPILEUP | 1 → 1 | 6 GB → 1 GB | 8h → 3h | $0.25 |
| SNPEFF_SNPEFF | 6 → 3 | 36 GB → 5 GB | 16h → 15m | $0.21 |
| MPILEUP_NORMAL:SAMTOOLS_MPILEUP | 1 → 1 | 6 GB → 1 GB | 8h → 2h | $0.20 |
| TIDDIT_TUMOR:TIDDIT_SV | 6 → 3 | 36 GB → 12 GB | 16h → 30m | $0.16 |
| BAM_VARIANT_CALLING_SINGLE_TIDDIT:TIDDIT_SV | 6 → 3 | 36 GB → 12 GB | 16h → 15m | $0.14 |
| BAM_VARIANT_CALLING_FREEBAYES:FREEBAYES | 1 → 1 | 6 GB → 2 GB | 8h → 2h | $0.13 |
| GETPILEUPSUMMARIES_TUMOR | 2 → 2 | 12 GB → 3 GB | 8h → 45m | $0.11 |
| BAM_VARIANT_CALLING_CNVKIT:CNVKIT_BATCH | 12 → 8 | 36 GB → 16 GB | 8h → 15m | $0.11 |
| TIDDIT_NORMAL:TIDDIT_SV | 6 → 3 | 36 GB → 12 GB | 16h → 15m | $0.11 |
| GETPILEUPSUMMARIES_NORMAL | 2 → 2 | 12 GB → 3 GB | 8h → 45m | $0.10 |
| MERGE_CRAM | 2 → 2 | 12 GB → 1 GB | 8h → 15m | $0.07 |
| MUSE_CALL | 12 → 12 | 72 GB → 38 GB | 32h → 15m | $0.06 |
| ASCAT | 6 → 6 | 36 GB → 5 GB | 16h → 15m | $0.06 |
| MANTA_SOMATIC | 10 → 10 | 8 GB → 2 GB | 16h → 45m | $0.06 |
| FASTQC | 4 → 3 | 4 GB → 2 GB | 16h → 30m | $0.05 |
| GATK4_APPLYBQSR | 2 → 2 | 4 GB → 3 GB | 8h → 2h | $0.05 |
| CNVKIT_GENEMETRICS | 2 → 2 | 12 GB → 2 GB | 8h → 15m | $0.04 |
| MANTA_GERMLINE | 10 → 10 | 8 GB → 1 GB | 16h → 30m | $0.04 |
| STRELKA_SOMATIC | 10 → 10 | 8 GB → 2 GB | 16h → 30m | $0.04 |

Savings assume the same run time at the smaller allocation, which holds for memory and for CPU-bound processes that were not using the extra cores. Validate on one real run before rolling out; a process at 100% CPU efficiency will slow down if you cut its cores.
