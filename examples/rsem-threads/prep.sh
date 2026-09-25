#!/usr/bin/env bash
# Build chr1 references and the STAR transcriptome BAM exactly as nf-core/rnaseq 3.22.0 does
# for --aligner star_rsem (STAR_ALIGN_IGENOMES args from subworkflows/local/align_star/nextflow.config,
# RSEM_PREPAREREFERENCE with no extra args). Only difference: chr1 instead of the whole genome.
set -euo pipefail
cd "$(dirname "$0")"
STAR_IMG=quay.io/biocontainers/mulled-v2-1fa26d1ce03c295fe2fdcf85831a92fbcbd7e8c2:59cdd445419f14abac76b31dd0d71217994cbcc9-0
RSEM_IMG=community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32
run() { local img=$1; shift; podman run --rm --userns=keep-id -v "$PWD":/w -w /w "$img" bash -c "$*" 2> >(grep -v 'graph driver' >&2); }

# chr1 genome + GTF
[ -s ref/chr1.fa ] || run $RSEM_IMG "samtools faidx ref/genome.fa && samtools faidx ref/genome.fa 1 > ref/chr1.fa"
[ -s ref/chr1.gtf ] || awk -F'\t' '$1=="1"' ref/genes.gtf > ref/chr1.gtf

# STAR index (STAR 2.6.1d, same version as STAR_ALIGN_IGENOMES)
if [ ! -s star/SA ]; then
  mkdir -p star
  run $STAR_IMG "STAR --runMode genomeGenerate --genomeDir star --genomeFastaFiles ref/chr1.fa --sjdbGTFfile ref/chr1.gtf --sjdbOverhang 100 --runThreadN 12 --outFileNamePrefix star/"
fi

# RSEM reference (RSEM 1.3.1, same as RSEM_CALCULATEEXPRESSION in 3.22.0)
if [ ! -s rsem/genome.grp ]; then
  mkdir -p rsem
  run $RSEM_IMG "rsem-prepare-reference --gtf ref/chr1.gtf --num-threads 12 ref/chr1.fa rsem/genome"
fi

# Transcriptome BAM (generateStarAlignArgs(..., 'rsem') + module defaults)
if [ ! -s aln/S.Aligned.toTranscriptome.out.bam ]; then
  mkdir -p aln; chmod 777 aln
  run $STAR_IMG "STAR --genomeDir star --readFilesIn mt/R1.fq.gz mt/R2.fq.gz --runThreadN 12 \
    --outFileNamePrefix aln/S. --sjdbGTFfile ref/chr1.gtf --outSAMattrRGline ID:S 'SM:S' \
    --quantMode TranscriptomeSAM --outSAMtype BAM Unsorted --outSAMattributes NH HI AS NM MD \
    --readFilesCommand zcat --outSAMunmapped Within --outFilterType BySJout --outFilterMultimapNmax 20 \
    --outFilterMismatchNmax 999 --outFilterMismatchNoverLmax 0.04 --alignIntronMin 20 --alignIntronMax 1000000 \
    --alignMatesGapMax 1000000 --alignSJoverhangMin 8 --alignSJDBoverhangMin 1 --sjdbScore 1"
fi
echo PREP_DONE
