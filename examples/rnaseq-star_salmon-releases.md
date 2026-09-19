# nf-audit compare

- rates: `seqera-compute` ($0.1000/CPU-h, $0.0250/GiB-h)
- runs: 25

## Runs

| run | nextflow | fusion | tasks | failed | wall | CPU-h req | GiB-h req | cost | metered | unused | top process |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 3.1 salmon 2021-05-14 | 21.04.0 | ? | 285 | 0 | 15h 5m 15s | 555 | 3328 | $138.66 | 285/285 | 67% | STAR_ALIGN 22% |
| 3.2 salmon 2021-06-18 | 21.04.0 | ? | 285 | 0 | 13h 49m 33s | 573 | 3435 | $143.14 | 285/285 | 68% | STAR_ALIGN 21% |
| 3.3 salmon 2021-07-29 | 21.04.0 | ? | 285 | 0 | 16h 48m 1s | 551 | 3304 | $137.67 | 285/285 | 67% | STAR_ALIGN 22% |
| 3.7 salmon 2022-05-03 | 22.04.0 | ? | 286 | 1 | 10h 39m 13s | 562 | 3371 | $140.45 | 285/286 | 67% | STAR_ALIGN 21% |
| 3.8 salmon 2022-05-25 | 22.04.2 | ? | 285 | 0 | 8h 51m 24s | 537 | 3223 | $134.30 | 285/285 | 67% | STAR_ALIGN 21% |
| 3.8.1 salmon 2022-05-27 | 22.04.2 | ? | 285 | 0 | 7h 22m 35s | 542 | 3253 | $135.52 | 285/285 | 67% | STAR_ALIGN_IGENOMES 21% |
| 3.9 salmon 2022-09-30 | 22.09.7.edge | ? | 285 | 2 | 9h 39m 2s | 489 | 2933 | $122.19 | 283/285 | 63% | STAR_ALIGN_IGENOMES 23% |
| 3.10.1 salmon 2023-01-05 | 22.10.4 | ? | 277 | 0 | 10h 5m 21s | 480 | 2880 | $120.00 | 277/277 | 63% | STAR_ALIGN_IGENOMES 24% |
| 3.11.1 salmon 2023-03-31 | 23.03.0-edge | ? | 277 | 0 | 4h 18m 50s | 337 | 2023 | $84.30 | 277/277 | 64% | STAR_ALIGN_IGENOMES 23% |
| 3.11.2 salmon 2023-04-25 | 23.04.1 | ? | 277 | 0 | 4h 14m 34s | 330 | 1982 | $82.58 | 277/277 | 64% | STAR_ALIGN_IGENOMES 24% |
| 3.12.0 salmon 2023-06-02 | 23.04.1 | ? | 277 | 0 | 4h 5m 29s | 338 | 2031 | $84.62 | 277/277 | 64% | STAR_ALIGN_IGENOMES 23% |
| 3.14.0 salmon 2024-01-08 | 23.10.0 | ? | 276 | 0 | 4h 56m 39s | 326 | 1955 | $81.44 | 276/276 | 66% | STAR_ALIGN_IGENOMES 23% |
| 3.15.0 salmon 2024-09-05 | 24.04.4 | 2.3 | 283 | 0 | 6h 48m 43s | 306 | 1836 | $76.51 | 283/283 | 66% | STAR_ALIGN_IGENOMES 24% |
| 3.15.1 salmon 2024-09-16 | 24.04.4 | 2.3 | 283 | 0 | 5h 8m 46s | 317 | 1901 | $79.23 | 283/283 | 66% | STAR_ALIGN_IGENOMES 24% |
| 3.16.0 salmon 2024-10-02 | 24.04.4 | 2.3 | 283 | 0 | 4h 32m 49s | 307 | 1839 | $76.63 | 283/283 | 66% | STAR_ALIGN_IGENOMES 24% |
| 3.16.1 salmon 2024-10-16 | 24.04.4 | 2.3 | 283 | 0 | 5h 1m 52s | 321 | 1926 | $80.24 | 283/283 | 65% | STAR_ALIGN_IGENOMES 24% |
| 3.17.0 salmon 2024-10-24 | 24.04.4 | 2.3 | 283 | 0 | 5h 12m 17s | 306 | 1833 | $76.38 | 283/283 | 67% | STAR_ALIGN_IGENOMES 24% |
| 3.18.0 salmon 2024-12-20 | 24.10.3 | 2.4 | 299 | 0 | 4h 43m 23s | 316 | 1897 | $79.03 | 299/299 | 67% | STAR_ALIGN_IGENOMES 24% |
| 3.22.0 salmon 2025-11-27 | 25.04.8 | 2.4 | 303 | 0 | 5h 11m 27s | 311 | 1864 | $77.66 | 303/303 | 64% | STAR_ALIGN_IGENOMES 31% |
| 3.22.1 salmon 2025-12-05 | 25.04.8 | 2.4 | 303 | 0 | 13h 30m 43s | 288 | 1727 | $71.94 | 303/303 | 65% | STAR_ALIGN_IGENOMES 30% |
| 3.22.2 salmon 2025-12-12 | 25.04.8 | 2.4 | 303 | 0 | 10h 22m 33s | 378 | 2267 | $94.45 | 303/303 | 64% | STAR_ALIGN_IGENOMES 31% |
| 3.23.0 salmon 2026-02-27 | 25.10.2 | 2.4 | 303 | 0 | 5h 10m 48s | 323 | 1940 | $80.84 | 303/303 | 64% | STAR_ALIGN_IGENOMES 31% |
| 3.24.0 salmon 2026-04-09 | 25.10.4 | 2.4 | 303 | 0 | 9h 39m 11s | 329 | 1976 | $82.33 | 303/303 | 66% | STAR_ALIGN_IGENOMES 31% |
| 3.25.0 salmon 2026-04-24 | 25.10.4 | 2.4 | 327 | 0 | 14h 34m 34s | 317 | 1903 | $79.29 | 327/327 | 66% | STAR_ALIGN_IGENOMES 30% |
| 3.26.0 salmon 2026-05-07 | 25.10.5 | 2.4 | 328 | 0 | 4h 28m 37s | 246 | 1401 | $59.64 | 328/328 | 62% | STAR_ALIGN 27% |

Sources:

- 3.1 salmon 2021-05-14 — `data/rnaseq/results-0fcbb0ac491ecb8a80ef879c4f3dad5f869021f9/aligner_star_salmon/pipeline_info/execution_report_2021-05-14_00-07-33.html`
- 3.2 salmon 2021-06-18 — `data/rnaseq/results-b3ff92bc54363faf17d820689a8e9074ffd99045/aligner_star_salmon/pipeline_info/execution_report_2021-06-18_14-06-38.html`
- 3.3 salmon 2021-07-29 — `data/rnaseq/results-8094c42add6dcdf69ce54dfdec957789c37ae903/aligner_star_salmon/pipeline_info/execution_report_2021-07-29_13-41-41.html`
- 3.7 salmon 2022-05-03 — `data/rnaseq/results-e0dfce9af5c2299bcc2b8a74b6559ce055965455/aligner_star_salmon/pipeline_info/execution_report_2022-05-03_11-19-34.html`
- 3.8 salmon 2022-05-25 — `data/rnaseq/results-6995330476244a6bffe55ddcbe50b8ed5cf6c2e2/aligner_star_salmon/pipeline_info/execution_report_2022-05-25_09-53-32.html`
- 3.8.1 salmon 2022-05-27 — `data/rnaseq/results-89bf536ce4faa98b4d50a8ec0a0343780bc62e0a/aligner_star_salmon/pipeline_info/execution_report_2022-05-27_16-42-32.html`
- 3.9 salmon 2022-09-30 — `data/rnaseq/results-e049f51f0214b2aef7624b9dd496a404a7c34d14/aligner_star_salmon/pipeline_info/execution_report_2022-09-30_20-29-46.html`
- 3.10.1 salmon 2023-01-05 — `data/rnaseq/results-6e1e448f535ccf34d11cc691bb241cfd6e60a647/aligner_star_salmon/pipeline_info/execution_report_2023-01-05_12-20-46.html`
- 3.11.1 salmon 2023-03-31 — `data/rnaseq/results-287afcfe30a93de77e9b7cf70a1085f58c9525d8/aligner_star_salmon/pipeline_info/execution_report_2023-03-31_16-12-02.html`
- 3.11.2 salmon 2023-04-25 — `data/rnaseq/results-5671b65af97fe78a2f9b4d05d850304918b1b86e/aligner_star_salmon/pipeline_info/execution_report_2023-04-25_10-57-49.html`
- 3.12.0 salmon 2023-06-02 — `data/rnaseq/results-3bec2331cac2b5ff88a1dc71a21fab6529b57a0f/aligner_star_salmon/pipeline_info/execution_report_2023-06-02_15-44-44.html`
- 3.14.0 salmon 2024-01-08 — `data/rnaseq/results-b89fac32650aacc86fcda9ee77e00612a1d77066/aligner_star_salmon/pipeline_info/execution_report_2024-01-08_17-25-43.html`
- 3.15.0 salmon 2024-09-05 — `data/rnaseq/results-4e34945f6ca86621a08e7d573cd6b4fbb7fb1f0e/aligner_star_salmon/pipeline_info/execution_report_2024-09-05_08-48-53.html`
- 3.15.1 salmon 2024-09-16 — `data/rnaseq/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_salmon/pipeline_info/execution_report_2024-09-16_16-33-21.html`
- 3.16.0 salmon 2024-10-02 — `data/rnaseq/results-33df0c05ef99d1d9af97d1d74681ab0452612abb/aligner_star_salmon/pipeline_info/execution_report_2024-10-02_15-10-02.html`
- 3.16.1 salmon 2024-10-16 — `data/rnaseq/results-1f3f64dac72d5ae8c15f19246fa5f564c9e063d7/aligner_star_salmon/pipeline_info/execution_report_2024-10-16_12-56-01.html`
- 3.17.0 salmon 2024-10-24 — `data/rnaseq/results-00f924cf92a986a842bb352b3c4ae379c773c989/aligner_star_salmon/pipeline_info/execution_report_2024-10-24_11-33-53.html`
- 3.18.0 salmon 2024-12-20 — `data/rnaseq/results-b96a75361a4f1d49aa969a2b1c68e3e607de06e8/aligner_star_salmon/pipeline_info/execution_report_2024-12-20_16-55-39.html`
- 3.22.0 salmon 2025-11-27 — `data/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_salmon/pipeline_info/execution_report_2025-11-27_10-50-37.html`
- 3.22.1 salmon 2025-12-05 — `data/rnaseq/results-759dbf125280e6faeaa4194dd74f872d224fc327/aligner_star_salmon/pipeline_info/execution_report_2025-12-05_18-27-02.html`
- 3.22.2 salmon 2025-12-12 — `data/rnaseq/results-3816d48abd9fab2eee41775b60b4eb8745e1fcaa/aligner_star_salmon/pipeline_info/execution_report_2025-12-12_10-02-21.html`
- 3.23.0 salmon 2026-02-27 — `data/rnaseq/results-13328f73915ceda9bb794caee2f662230c5a33b6/aligner_star_salmon/pipeline_info/execution_report_2026-02-27_16-55-49.html`
- 3.24.0 salmon 2026-04-09 — `data/rnaseq/results-47b3b0d3daad69e99d45c9e2dd8db19ee28307a1/aligner_star_salmon/pipeline_info/execution_report_2026-04-09_16-53-04.html`
- 3.25.0 salmon 2026-04-24 — `data/rnaseq/results-891468c53574d531ae3f75b3a558552839cf973d/aligner_star_salmon/pipeline_info/execution_report_2026-04-24_16-05-23.html`
- 3.26.0 salmon 2026-05-07 — `data/rnaseq/results-e7ca46272c8f9d5ceee3f71759f4ba551d3217a4/aligner_star_salmon/pipeline_info/execution_report_2026-05-07_15-32-01.html`

## Cost share by process (top 12, % of each run's cost)

| process | 3.1 salmon 2021-05-14 | 3.2 salmon 2021-06-18 | 3.3 salmon 2021-07-29 | 3.7 salmon 2022-05-03 | 3.8 salmon 2022-05-25 | 3.8.1 salmon 2022-05-27 | 3.9 salmon 2022-09-30 | 3.10.1 salmon 2023-01-05 | 3.11.1 salmon 2023-03-31 | 3.11.2 salmon 2023-04-25 | 3.12.0 salmon 2023-06-02 | 3.14.0 salmon 2024-01-08 | 3.15.0 salmon 2024-09-05 | 3.15.1 salmon 2024-09-16 | 3.16.0 salmon 2024-10-02 | 3.16.1 salmon 2024-10-16 | 3.17.0 salmon 2024-10-24 | 3.18.0 salmon 2024-12-20 | 3.22.0 salmon 2025-11-27 | 3.22.1 salmon 2025-12-05 | 3.22.2 salmon 2025-12-12 | 3.23.0 salmon 2026-02-27 | 3.24.0 salmon 2026-04-09 | 3.25.0 salmon 2026-04-24 | 3.26.0 salmon 2026-05-07 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| STAR_ALIGN_IGENOMES | · | · | · | · | · | 21 | 23 | 24 | 23 | 24 | 23 | 23 | 24 | 24 | 24 | 24 | 24 | 24 | 31 | 30 | 31 | 31 | 31 | 30 | · |
| TRIMGALORE | 20 | 21 | 20 | 19 | 20 | 20 | 15 | 17 | 18 | 17 | 17 | 17 | 17 | 17 | 17 | 17 | 17 | 17 | 18 | 18 | 17 | 17 | 16 | 17 | 2 |
| QUALIMAP_RNASEQ | 18 | 18 | 17 | 16 | 17 | 17 | 18 | 19 | 20 | 20 | 20 | 21 | 23 | 23 | 22 | 22 | 23 | 22 | 9 | 9 | 11 | 9 | 8 | 8 | 12 |
| PICARD_MARKDUPLICATES | 9 | 9 | 9 | 13 | 9 | 9 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 11 | 11 | 11 | 11 | 11 | 11 | 15 |
| STAR_ALIGN | 22 | 21 | 22 | 21 | 21 | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | · | 27 |
| RSEQC_READDUPLICATION | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 3 | 4 | 4 | 4 | 6 |
| QUANTIFY_SALMON:SALMON_QUANT | 4 | 4 | 4 | 5 | 6 | 7 | 8 | 4 | 4 | 4 | 5 | · | · | · | · | · | · | · | · | · | · | · | · | · | · |
| DUPRADAR | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 3 | 4 | 4 | 3 | 3 | 6 | 7 | 4 |
| BEDTOOLS_GENOMECOV | 3 | 3 | 3 | 3 | 3 | 3 | 4 | 4 | 4 | 4 | 4 | 4 | · | · | · | · | · | · | · | · | · | · | · | · | · |
| QUANTIFY_PSEUDO_ALIGNMENT:SALMON_QUANT | · | · | · | · | · | · | · | · | · | · | · | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 5 | 4 | 4 | 4 | 6 |
| FASTQC | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 3 | 2 | 2 | 1 | 1 | 1 | 1 |
| RSEQC_READDISTRIBUTION | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 3 |

`·` = process absent from that run.
