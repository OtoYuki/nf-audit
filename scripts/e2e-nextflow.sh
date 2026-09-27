#!/usr/bin/env bash
# End to end against real Nextflow: run testdata/nextflow-local/main.nf, audit it, apply the
# generated fragment with `-c`, and check that Nextflow ran the lowered requests and kept the
# escalating process's retry escalation.
#
# Usage: scripts/e2e-nextflow.sh [--update]
#   --update  also copy the fresh trace and report over testdata/nextflow-local/
# Needs podman or docker. NXF_IMAGE overrides the Nextflow image.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
IMAGE="${NXF_IMAGE:-docker.io/nextflow/nextflow:25.04.8}"
ENGINE=$(command -v docker || command -v podman) || { echo "needs podman or docker" >&2; exit 2; }

cargo build --release --quiet
W=$(mktemp -d)
# Files written by a root container (docker) may not be removable; that is not a failure.
trap 'rm -rf "$W" 2>/dev/null || true' EXIT
cp testdata/nextflow-local/main.nf testdata/nextflow-local/nextflow.config "$W/"
nxf() { "$ENGINE" run --rm -v "$W:/w:Z" -w /w "$IMAGE" nextflow "$@"; }

nxf run main.nf -ansi-log false >/dev/null
"$ROOT/target/release/nf-audit" analyze \
  --trace "$W/pipeline_info/execution_trace.txt" --report "$W/pipeline_info/execution_report.html" \
  --config-out "$W/nf-audit.config" >"$W/report.md"
grep -q "Left as they are.*ESCALATES" "$W/report.md" || { echo "ESCALATES was not left alone" >&2; exit 1; }
if grep -q ESCALATES "$W/nf-audit.config"; then echo "ESCALATES is in the fragment" >&2; exit 1; fi

cat >"$W/fields.config" <<'EOF'
trace { fields = "name,status,attempt,cpus,memory,time"; file = "pipeline_info/applied.txt"; overwrite = true }
EOF
nxf run main.nf -ansi-log false -c nf-audit.config -c fields.config >/dev/null

# Every BUSY task ran at the fragment's values; the retried ESCALATES attempt still doubled.
busy=$(grep -c $'SUB:BUSY ([^)]*)\tCOMPLETED\t1\t2\t1 GB\t15m' "$W/pipeline_info/applied.txt" || true)
retry=$(grep -c $'SUB:ESCALATES (s1)\tCOMPLETED\t2\t2\t2 GB\t30m' "$W/pipeline_info/applied.txt" || true)
if [[ $busy -ne 3 || $retry -ne 1 ]]; then
  cat "$W/pipeline_info/applied.txt" >&2
  echo "fragment not applied as expected (BUSY rows: $busy/3, escalated retry: $retry/1)" >&2
  exit 1
fi

if [[ "${1:-}" == "--update" ]]; then
  cp "$W"/pipeline_info/execution_trace.txt "$W"/pipeline_info/execution_report.html testdata/nextflow-local/
fi
echo "e2e: Nextflow loaded the fragment; BUSY ran at 2 CPUs / 1 GB / 15m, ESCALATES kept its escalation"
