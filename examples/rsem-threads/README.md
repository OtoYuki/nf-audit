# Does `RSEM_CALCULATEEXPRESSION` use its 12 CPUs?

In nf-core/rnaseq's AWS full-size tests (`--aligner star_rsem`, 3.22.0 onward), `RSEM_CALCULATEEXPRESSION` requests 12 CPUs / 72 GiB. Its completed tasks average about one core (median 104%, interquartile range 94–131%, mean 120% over 45 tasks; range 25–511%) and peak at or below 9.45 GiB. This directory tests whether that is RSEM itself or the environment, by running the pipeline's own command line locally at several thread counts.

## What is reproduced

| piece | as in rnaseq 3.22.0 | here |
|---|---|---|
| RSEM | `community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32` (conda `rsem-1.3.3`; the binary prints "v1.3.1") | same image |
| RSEM command | `rsem-calculate-expression --num-threads N --temporary-folder ./tmp/ --alignments --strandedness reverse --paired-end --estimate-rspd --seed 1 <transcriptome.bam> <index> <prefix>` | same |
| RSEM reference | `rsem-prepare-reference --gtf genes.filtered.gtf --num-threads 12 genome.fa`, iGenomes Ensembl GRCh37 | same files, **chromosome 1 only**. The pipeline's `filter_gtf.py` keeps lines on sequences in the FASTA that carry a `transcript_id`; all 232,647 chr1 lines do, so `chr1.gtf` equals the filtered GTF restricted to chr1 |
| STAR | `STAR_ALIGN_IGENOMES`, STAR 2.6.1d, `generateStarAlignArgs(..., 'rsem')` | same image and arguments, chr1 index |
| reads | GM12878_REP1, full sample | the **7,792,588 read pairs that 3.22.0's own STAR run placed on chr1** (both mates), streamed from the published `GM12878_REP1.markdup.sorted.bam` |

Hardware: a laptop with an 8-core / 16-thread i7-11800H (so the 12-thread run relies on hyper-threading) and local NVMe. Each run is a container capped with `--cpus=N` (a CFS quota, not a cpuset). The runs pass RSEM's `--time` in addition to the pipeline's arguments.

## Results

`runs/summary.tsv`, one run per thread count:

| `--num-threads` | wall | speed-up | avg. CPU | CPUs × wall | cgroup `memory.peak`* (decimal GB) |
|---:|---:|---:|---:|---:|---:|
| 1 | 1,699 s | 1.00× | 99% | 1,699 CPU·s | 2.7 GB |
| 2 | 976 s | 1.74× | 185% | 1,952 CPU·s | 5.2 GB |
| 4 | 574 s | 2.96× | 335% | 2,297 CPU·s | 3.5 GB |
| 8 | 440 s | 3.86× | 545% | 3,522 CPU·s | 4.8 GB |
| 12 | 389 s | 4.36× | 757% | 4,673 CPU·s | 5.2 GB |

\* `memory.peak` of the container's cgroup, which includes page cache, so it is not RSEM's resident memory. The megatest `peak_rss` values (≤9.45 GiB) are the ones to size memory by.

- **Output:** `S.genes.results` and `S.isoforms.results` have the same sha256 at every thread count (`runs/hashes.txt`).
- **STAR:** 96.55% of input pairs uniquely mapped on chr1 (`runs/star_chr1_log.txt`).
- **Parse step alone** (`rsem_parse_only.sh`, output in `runs/parse_only_p1.txt`): `rsem-parse-alignments` at 1 CPU took 92.5–97.7 s over two runs for 18 M alignment entries (~184–195 k entries/s).
  - This is the single-threaded step in which three timed-out megatest tasks logged their last line: 3.22.1 `Parsed 245000000 entries`, 3.23.0 `255000000`, 3.24.0 `73000000`.
  - Over their 16 h, those tasks averaged about 4.3 k, 4.4 k and 1.3 k entries/s.
- **I/O** (`runs/io_p12.txt`, a separate 12-thread run): the task read 37.42 GB and wrote 4.52 GB for a 1.71 GB input BAM. Most of the reading is `rsem-run-em`, 33.09 GB.

## What it does and does not show

**Shows:**
- On local disk, the whole RSEM task speeds up with threads (the serial parse step is about 95 s of it). Only the whole task was timed, not the EM step alone. Returns diminish: 4 threads reach 68% of the 12-thread speed for 49% of the reserved CPU time (CPUs × wall).
- RSEM re-reads heavily: 21.9× the input size in a separate 12-thread run here. The megatest tasks read 17.6–20.7× theirs on 44 of 45 completed tasks (their BAM sizes, 20.3–25.8 GB, are in each task's rendered script); the exception, 3.25.0 `K562_REP2`, read 1.3×. So the read volume is a similar pattern, and what differs on the test infrastructure is speed.

**Does not show:**
- Why the same command averages about one core on the megatest infrastructure.
- How a full sample, or the whole transcriptome, scales. This is chr1 only, one sample, one machine, one run per point.

## Reproduce

```bash
./fetch_ref.sh              # iGenomes GRCh37 genome + GTF, both containers
./reads_from_megatest.sh    # chr1 read pairs from the 3.22.0 megatest BAM (streams only the head of the file)
./prep.sh                   # chr1 FASTA/GTF, STAR index, RSEM reference, transcriptome BAM
./rsem_threads.sh           # THREADS="1 2 4 8 12" by default
./rsem_io.sh 12             # optional: /proc/<pid>/io sampling
./rsem_parse_only.sh        # optional: the parse step alone, 1 CPU
```

Needs podman and about 25 GB of disk. The working directory reached 22 GB: about 10 GB is the six runs' ~1.6 GB `S.transcript.bam` files, and the rest is the STAR index, alignments, reads and parse-only temp files.

The files in `runs/` come from these scripts: `summary.tsv`, `hashes.txt` and `p*.time` from `rsem_threads.sh`, `io_p12.txt` from `rsem_io.sh` (plus two hand-added comment lines), and `parse_only_p1.txt` from `rsem_parse_only.sh` (the first of its two runs was typed in by hand from the same command). Do not run it under a RAM-backed `/tmp`: the first attempt at this silently produced truncated BAMs when the tmpfs filled.
