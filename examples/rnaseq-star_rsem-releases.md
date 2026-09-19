# nf-audit compare

- rates: `seqera-compute` ($0.1000/CPU-h, $0.0250/GiB-h)
- runs: 20

## Runs

| run | nextflow | fusion | tasks | failed | wall | CPU-h req | GiB-h req | cost | metered | unused | top process |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 3.1 rsem 2021-05-14 | 21.04.0 | ? | 273 | 0 | 16h 33m 44s | 750 | 4501 | $187.53 | 273/273 | 64% | RSEM_CALCULATEEXPRESSION 42% |
| 3.2 rsem 2021-06-18 | 21.04.0 | ? | 273 | 0 | 15h 52m 58s | 773 | 4640 | $193.35 | 273/273 | 65% | RSEM_CALCULATEEXPRESSION 42% |
| 3.3 rsem 2021-07-29 | 21.04.0 | ? | 273 | 0 | 17h 20m 12s | 738 | 4431 | $184.62 | 273/273 | 64% | RSEM_CALCULATEEXPRESSION 41% |
| 3.7 rsem 2022-05-03 | 22.04.0 | ? | 273 | 0 | 13h 32m 45s | 756 | 4533 | $188.88 | 273/273 | 65% | RSEM_CALCULATEEXPRESSION 42% |
| 3.8 rsem 2022-05-25 | 22.04.2 | ? | 273 | 0 | 9h 59m 35s | 710 | 4260 | $177.51 | 273/273 | 65% | RSEM_CALCULATEEXPRESSION 39% |
| 3.8.1 rsem 2022-05-27 | 22.04.2 | ? | 273 | 0 | 9h 43m 52s | 736 | 4414 | $183.90 | 273/273 | 65% | RSEM_CALCULATEEXPRESSION 39% |
| 3.9 rsem 2022-09-30 | 22.09.7.edge | ? | 273 | 0 | 12h 42s | 672 | 4030 | $167.90 | 273/273 | 61% | RSEM_CALCULATEEXPRESSION 43% |
| 3.10.1 rsem 2023-01-05 | 22.10.4 | ? | 265 | 0 | 11h 26m 19s | 658 | 3946 | $164.43 | 265/265 | 60% | RSEM_CALCULATEEXPRESSION 45% |
| 3.11.1 rsem 2023-03-31 | 23.03.0-edge | ? | 265 | 0 | 9h 46m 30s | 785 | 4711 | $196.28 | 265/265 | 67% | RSEM_CALCULATEEXPRESSION 66% |
| 3.11.2 rsem 2023-04-25 | 23.04.1 | ? | 265 | 0 | 9h 51m 53s | 741 | 4443 | $185.14 | 265/265 | 67% | RSEM_CALCULATEEXPRESSION 64% |
| 3.12.0 rsem 2023-06-02 | 23.04.1 | ? | 265 | 0 | 9h 52m 7s | 775 | 4649 | $193.71 | 265/265 | 68% | RSEM_CALCULATEEXPRESSION 66% |
| 3.14.0 rsem 2024-01-08 | 23.10.0 | ? | 264 | 0 | 10h 32m 6s | 792 | 4751 | $197.97 | 264/264 | 69% | RSEM_CALCULATEEXPRESSION 68% |
| 3.15.0 rsem 2024-09-05 | 24.04.4 | 2.3 | 271 | 0 | 10h 2m 21s | 685 | 4109 | $171.20 | 271/271 | 68% | RSEM_CALCULATEEXPRESSION 66% |
| 3.15.1 rsem 2024-09-17 | 24.04.4 | 2.3 | 271 | 0 | 6h 16m 16s | 728 | 4368 | $181.99 | 271/271 | 68% | RSEM_CALCULATEEXPRESSION 67% |
| 3.16.0 rsem 2024-10-02 | 24.04.4 | 2.3 | 271 | 0 | 14h 10m 7s | 656 | 3934 | $163.94 | 271/271 | 68% | RSEM_CALCULATEEXPRESSION 64% |
| 3.16.1 rsem 2024-10-16 | 24.04.4 | 2.3 | 271 | 0 | 12h 43m 6s | 708 | 4251 | $177.12 | 271/271 | 68% | RSEM_CALCULATEEXPRESSION 65% |
| 3.17.0 rsem 2024-10-24 | 24.04.4 | 2.3 | 271 | 0 | 11h 2m 57s | 725 | 4352 | $181.34 | 271/271 | 69% | RSEM_CALCULATEEXPRESSION 67% |
| 3.18.0 rsem 2024-12-20 | 24.10.3 | 2.4 | 292 | 5 | 21h 3m 48s | 1823 | 10938 | $455.75 | 287/292 | 78% | RSEM_CALCULATEEXPRESSION 86% |
| 3.22.0 rsem 2025-11-27 | 25.04.8 | 2.4 | 301 | 0 | 12h 57m 8s | 1177 | 7062 | $294.25 | 301/301 | 84% | RSEM_CALCULATEEXPRESSION 75% |
| 3.22.2 rsem 2025-12-12 | 25.04.8 | 2.4 | 301 | 0 | 16h 39m 3s | 1066 | 6394 | $266.43 | 301/301 | 81% | RSEM_CALCULATEEXPRESSION 69% |

Sources:

- 3.1 rsem 2021-05-14 — `data/rnaseq/results-0fcbb0ac491ecb8a80ef879c4f3dad5f869021f9/aligner_star_rsem/pipeline_info/execution_report_2021-05-14_00-07-35.html`
- 3.2 rsem 2021-06-18 — `data/rnaseq/results-b3ff92bc54363faf17d820689a8e9074ffd99045/aligner_star_rsem/pipeline_info/execution_report_2021-06-18_14-07-00.html`
- 3.3 rsem 2021-07-29 — `data/rnaseq/results-8094c42add6dcdf69ce54dfdec957789c37ae903/aligner_star_rsem/pipeline_info/execution_report_2021-07-29_13-47-30.html`
- 3.7 rsem 2022-05-03 — `data/rnaseq/results-e0dfce9af5c2299bcc2b8a74b6559ce055965455/aligner_star_rsem/pipeline_info/execution_report_2022-05-03_11-19-16.html`
- 3.8 rsem 2022-05-25 — `data/rnaseq/results-6995330476244a6bffe55ddcbe50b8ed5cf6c2e2/aligner_star_rsem/pipeline_info/execution_report_2022-05-25_09-53-33.html`
- 3.8.1 rsem 2022-05-27 — `data/rnaseq/results-89bf536ce4faa98b4d50a8ec0a0343780bc62e0a/aligner_star_rsem/pipeline_info/execution_report_2022-05-27_16-42-07.html`
- 3.9 rsem 2022-09-30 — `data/rnaseq/results-e049f51f0214b2aef7624b9dd496a404a7c34d14/aligner_star_rsem/pipeline_info/execution_report_2022-09-30_20-29-45.html`
- 3.10.1 rsem 2023-01-05 — `data/rnaseq/results-6e1e448f535ccf34d11cc691bb241cfd6e60a647/aligner_star_rsem/pipeline_info/execution_report_2023-01-05_12-22-57.html`
- 3.11.1 rsem 2023-03-31 — `data/rnaseq/results-287afcfe30a93de77e9b7cf70a1085f58c9525d8/aligner_star_rsem/pipeline_info/execution_report_2023-03-31_16-12-02.html`
- 3.11.2 rsem 2023-04-25 — `data/rnaseq/results-5671b65af97fe78a2f9b4d05d850304918b1b86e/aligner_star_rsem/pipeline_info/execution_report_2023-04-25_10-57-48.html`
- 3.12.0 rsem 2023-06-02 — `data/rnaseq/results-3bec2331cac2b5ff88a1dc71a21fab6529b57a0f/aligner_star_rsem/pipeline_info/execution_report_2023-06-02_15-44-41.html`
- 3.14.0 rsem 2024-01-08 — `data/rnaseq/results-b89fac32650aacc86fcda9ee77e00612a1d77066/aligner_star_rsem/pipeline_info/execution_report_2024-01-08_17-25-45.html`
- 3.15.0 rsem 2024-09-05 — `data/rnaseq/results-4e34945f6ca86621a08e7d573cd6b4fbb7fb1f0e/aligner_star_rsem/pipeline_info/execution_report_2024-09-05_08-48-52.html`
- 3.15.1 rsem 2024-09-17 — `data/rnaseq/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_rsem/pipeline_info/execution_report_2024-09-17_16-06-18.html`
- 3.16.0 rsem 2024-10-02 — `data/rnaseq/results-33df0c05ef99d1d9af97d1d74681ab0452612abb/aligner_star_rsem/pipeline_info/execution_report_2024-10-02_15-10-01.html`
- 3.16.1 rsem 2024-10-16 — `data/rnaseq/results-1f3f64dac72d5ae8c15f19246fa5f564c9e063d7/aligner_star_rsem/pipeline_info/execution_report_2024-10-16_12-55-57.html`
- 3.17.0 rsem 2024-10-24 — `data/rnaseq/results-00f924cf92a986a842bb352b3c4ae379c773c989/aligner_star_rsem/pipeline_info/execution_report_2024-10-24_11-33-52.html`
- 3.18.0 rsem 2024-12-20 — `data/rnaseq/results-b96a75361a4f1d49aa969a2b1c68e3e607de06e8/aligner_star_rsem/pipeline_info/execution_report_2024-12-20_16-55-31.html`
- 3.22.0 rsem 2025-11-27 — `data/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info/execution_report_2025-11-27_10-50-37.html`
- 3.22.2 rsem 2025-12-12 — `data/rnaseq/results-3816d48abd9fab2eee41775b60b4eb8745e1fcaa/aligner_star_rsem/pipeline_info/execution_report_2025-12-12_10-02-14.html`

## Cost share by process (top 12, % of each run's cost)

| process | 3.1 rsem 2021-05-14 | 3.2 rsem 2021-06-18 | 3.3 rsem 2021-07-29 | 3.7 rsem 2022-05-03 | 3.8 rsem 2022-05-25 | 3.8.1 rsem 2022-05-27 | 3.9 rsem 2022-09-30 | 3.10.1 rsem 2023-01-05 | 3.11.1 rsem 2023-03-31 | 3.11.2 rsem 2023-04-25 | 3.12.0 rsem 2023-06-02 | 3.14.0 rsem 2024-01-08 | 3.15.0 rsem 2024-09-05 | 3.15.1 rsem 2024-09-17 | 3.16.0 rsem 2024-10-02 | 3.16.1 rsem 2024-10-16 | 3.17.0 rsem 2024-10-24 | 3.18.0 rsem 2024-12-20 | 3.22.0 rsem 2025-11-27 | 3.22.2 rsem 2025-12-12 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| RSEM_CALCULATEEXPRESSION | 42 | 42 | 41 | 42 | 39 | 39 | 43 | 45 | 66 | 64 | 66 | 68 | 66 | 67 | 64 | 65 | 67 | 86 | 75 | 69 |
| QUALIMAP_RNASEQ | 13 | 13 | 13 | 12 | 12 | 12 | 13 | 14 | 9 | 10 | 9 | 9 | 10 | 10 | 10 | 10 | 10 | 4 | 2 | 3 |
| TRIMGALORE | 14 | 15 | 15 | 14 | 15 | 15 | 12 | 13 | 8 | 8 | 7 | 7 | 8 | 7 | 8 | 8 | 7 | 3 | 5 | 5 |
| PICARD_MARKDUPLICATES | 7 | 7 | 7 | 7 | 7 | 7 | 8 | 8 | 5 | 5 | 5 | 4 | 5 | 4 | 5 | 5 | 5 | 2 | 3 | 4 |
| SALMON_QUANT | 3 | 3 | 3 | 5 | 5 | 6 | 6 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 1 | 1 | 2 |
| RSEQC_READDUPLICATION | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| BEDTOOLS_GENOMECOV | 2 | 2 | 3 | 2 | 3 | 2 | 3 | 3 | 2 | 2 | 2 | 2 | · | · | · | · | · | · | · | · |
| STAR_ALIGN_IGENOMES | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 7 | 9 |
| FASTQC | 1 | 1 | 1 | 1 | 1 | 1 | 2 | 2 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 1 | 1 |
| PRESEQ_LCEXTRAP | 3 | 3 | 4 | 3 | 4 | 3 | 1 | · | · | · | · | · | · | · | · | · | · | · | · | · |
| RSEQC_READDISTRIBUTION | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 1 | 1 |
| DUPRADAR | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |

`·` = process absent from that run.
