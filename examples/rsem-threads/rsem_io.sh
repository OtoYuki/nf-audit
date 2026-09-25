#!/usr/bin/env bash
# One RSEM run (same command as rsem_threads.sh, P threads) while sampling /proc/<pid>/io of every
# rsem-* / samtools process each 0.5 s; the last sample per pid ≈ its lifetime rchar/wchar
# (Nextflow's trace rchar/wchar come from the same /proc counters).
set -euo pipefail
cd "$(dirname "$0")"
P="${1:-12}"
RSEM_IMG=community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32
d=runs/io_p$P; podman unshare rm -rf "$d"; mkdir -p "$d"; chmod 777 "$d"
podman run --rm --cpus="$P" -v "$PWD":/w -w /w/$d $RSEM_IMG bash -c "
  ( while sleep 0.5; do for p in \$(pgrep -f 'rsem-|samtools'); do
      [ -r /proc/\$p/io ] && echo \"\$p \$(tr -d '\n' < /proc/\$p/cmdline | cut -c1-40) \$(grep -E '^(rchar|wchar)' /proc/\$p/io | awk '{printf \"%s \", \$2}')\"; done; done ) > io.samples &
  rsem-calculate-expression --num-threads $P --temporary-folder ./tmp/ --alignments \
      --strandedness reverse --paired-end --estimate-rspd --seed 1 --time \
      /w/aln/S.Aligned.toTranscriptome.out.bam /w/rsem/genome S > rsem.stdout 2> rsem.stderr
  kill %1 || true
" 2> >(grep -v 'graph driver' >&2)
python3 - "$d/io.samples" <<'EOF'
import sys
last = {}
for line in open(sys.argv[1]):
    f = line.split()
    if len(f) >= 4:
        last[f[0]] = (f[1], int(f[-2]), int(f[-1]))
for pid, (cmd, r, w) in sorted(last.items(), key=lambda x: -x[1][1]):
    print(f"{pid:>7} {cmd:40s} rchar {r/1e9:8.2f} GB  wchar {w/1e9:7.2f} GB")
# A parent's /proc/<pid>/io includes the counters of children it has reaped, so the
# rsem-calculate-expression (perl) row is the task total; summing the rows double-counts.
EOF
ls -la aln/S.Aligned.toTranscriptome.out.bam
