#!/usr/bin/env bash
# The reference nf-core/rnaseq 3.22.0 used for test_full (params: genome = GRCh37, iGenomes Ensembl),
# and the two containers of the tasks being reproduced. ~4 GB download.
set -euo pipefail
cd "$(dirname "$0")"
I=https://ngi-igenomes.s3.amazonaws.com/igenomes/Homo_sapiens/Ensembl/GRCh37
mkdir -p ref
[ -s ref/genome.fa ] || curl -sS -o ref/genome.fa $I/Sequence/WholeGenomeFasta/genome.fa
[ -s ref/genes.gtf ] || curl -sS -o ref/genes.gtf $I/Annotation/Genes/genes.gtf
podman pull -q community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32            # RSEM_CALCULATEEXPRESSION
podman pull -q quay.io/biocontainers/mulled-v2-1fa26d1ce03c295fe2fdcf85831a92fbcbd7e8c2:59cdd445419f14abac76b31dd0d71217994cbcc9-0  # STAR_ALIGN_IGENOMES
