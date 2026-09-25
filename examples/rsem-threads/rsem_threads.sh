#!/usr/bin/env bash
# Run RSEM_CALCULATEEXPRESSION's 3.22.0 command line (plus --time) at several --num-threads values,
# each in a container capped to that many CPUs with --cpus, and record wall time, user+sys CPU,
# the cgroup's memory.peak (includes page cache), RSEM's own per-step timing and output hashes.
set -euo pipefail
cd "$(dirname "$0")"
RSEM_IMG=community.wave.seqera.io/library/rsem_star:5acb4e8c03239c32
THREADS="${THREADS:-1 2 4 8 12}"
mkdir -p runs; chmod 777 runs
echo -e "threads\twall_s\tuser_s\tsys_s\tcpu_pct\tpeak_mem_bytes" > runs/summary.tsv   # one fresh table per invocation
for p in $THREADS; do
  d=runs/p$p; podman unshare rm -rf "$d"; mkdir -p "$d"; chmod 777 "$d"
  podman run --rm --userns=keep-id --cpus="$p" -v "$PWD":/w -w /w/$d $RSEM_IMG bash -c "
    s=\$(date +%s.%N)
    TIMEFORMAT='%U %S'
    { time rsem-calculate-expression --num-threads $p --temporary-folder ./tmp/ --alignments \
        --strandedness reverse --paired-end --estimate-rspd --seed 1 --time \
        /w/aln/S.Aligned.toTranscriptome.out.bam /w/rsem/genome S > rsem.stdout 2> rsem.stderr ; } 2> cpu.txt
    e=\$(date +%s.%N)
    echo \"\$s \$e\" > wall.txt
    cat /sys/fs/cgroup/memory.peak > peak.txt 2>/dev/null || echo NA > peak.txt
  " 2>&1 | grep -v 'graph driver' || true
  read s e < "$d/wall.txt"; read u sy < "$d/cpu.txt"
  wall=$(python3 -c "print(round($e-$s,1))"); pct=$(python3 -c "print(round(100*($u+$sy)/($e-$s)))")
  echo -e "$p\t$wall\t$u\t$sy\t$pct\t$(cat $d/peak.txt)" >> runs/summary.tsv
  tail -1 runs/summary.tsv
  tr '\n' ' ' < "$d/S.time" > "runs/p$p.time"; echo >> "runs/p$p.time"   # RSEM --time, one line
done
( cd runs && sha256sum p*/S.genes.results p*/S.isoforms.results ) > runs/hashes.txt
echo BENCH_DONE
