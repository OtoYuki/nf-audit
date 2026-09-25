#!/usr/bin/env bash
# Reads for the benchmark = the read pairs nf-core/rnaseq 3.22.0's own STAR run (full GRCh37 igenome)
# placed on chromosome 1 for GM12878_REP1. The published BAM is coordinate-sorted with chr1 first,
# so only the head of the file is streamed. Both mates must map to chr1; secondary/supplementary
# records are dropped so every read appears once per mate.
set -euo pipefail
cd "$(dirname "$0")"
IMG=community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32
URL=https://nf-core-awsmegatests.s3.eu-west-1.amazonaws.com/rnaseq/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/star_rsem/GM12878_REP1.markdup.sorted.bam
mkdir -p mt; chmod 777 mt
pr() { podman run --rm -i -v "$PWD":/w -w /w "$IMG" "$@" 2> >(grep -v 'graph driver' >&2); }

curl -sS "$URL" \
  | pr samtools view -h - \
  | awk -F'\t' '/^@/ {print; next} $3=="1" {seen=1; print; next} seen {exit}' \
  | pr samtools view -b -F 0x900 -o mt/chr1.bam - || true   # awk exits early by design; curl gets SIGPIPE
pr samtools view -c mt/chr1.bam
pr bash -c "samtools collate -u -O mt/chr1.bam mt/tmpcollate | samtools fastq -f 0x1 -1 mt/R1.fq.gz -2 mt/R2.fq.gz -s /dev/null -0 /dev/null -"
for f in mt/R1.fq.gz mt/R2.fq.gz; do echo "$f $(( $(gzip -dc $f | wc -l) / 4 )) reads"; done
echo READS_DONE
