#!/usr/bin/env bash
# Time rsem-parse-alignments on its own, at 1 CPU: the single-threaded first step of
# rsem-calculate-expression, with the arguments rsem-calculate-expression passes it for this input.
set -euo pipefail
cd "$(dirname "$0")"
RSEM_IMG=community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32
d=runs/parse_only; podman unshare rm -rf "$d"; mkdir -p "$d"; chmod 777 "$d"
podman run --rm --cpus=1 -v "$PWD":/w -w /w/$d $RSEM_IMG bash -c '
  mkdir -p tmp S.stat
  TIMEFORMAT="wall %R s  user %U s  sys %S s"
  { time rsem-parse-alignments /w/rsem/genome ./tmp/S S.stat/S /w/aln/S.Aligned.toTranscriptome.out.bam 3 -tag XM > parse.out 2>&1 ; } 2> time.txt
' 2> >(grep -v 'graph driver' >&2)
{ cat "$d/time.txt"; tail -2 "$d/parse.out"; } | tee -a runs/parse_only_p1.txt
