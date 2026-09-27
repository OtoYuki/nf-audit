#!/usr/bin/env bash
# Regenerate every nf-audit output under examples/ from the megatest files in data/.
#
# Usage: scripts/regen-examples.sh [--check]
#   --check   regenerate into a temp dir and diff against examples/; exit 1 on any difference.
#
# Needs the inputs first (109 files, about 367 MB): scripts/pull-megatests.sh --corpus
# Run from the repository root.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --quiet
BIN=./target/release/nf-audit

CHECK=0
[[ "${1:-}" == "--check" ]] && CHECK=1
OUT=examples
if [[ $CHECK -eq 1 ]]; then
  OUT=$(mktemp -d)
  trap 'rm -rf "$OUT"' EXIT
fi

# Every input must be present and byte-identical to the one the committed examples came from.
if ! sha256sum --quiet -c examples/corpus.sha256; then
  echo "inputs missing or changed; fetch them with scripts/pull-megatests.sh --corpus" >&2
  exit 2
fi

analyze() { # name trace-or-empty report top
  local name=$1 trace=$2 report=$3 top=$4
  local args=(analyze --report "$report" --top "$top" --config-out "$OUT/$name.nf-audit.config")
  [[ -n "$trace" ]] && args+=(--trace "$trace")
  "$BIN" "${args[@]}" >"$OUT/$name.md"
}

R=data/rnaseq
analyze rnaseq-3.15.1-star_salmon \
  $R/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_salmon/pipeline_info/execution_trace_2024-09-16_16-33-21.txt \
  $R/results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb/aligner_star_salmon/pipeline_info/execution_report_2024-09-16_16-33-21.html 20
analyze rnaseq-3.22.0-star_rsem \
  $R/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info/execution_trace_2025-11-27_10-50-37.txt \
  $R/results-c522f87e1406ede19f9df9593d00c3c404684790/aligner_star_rsem/pipeline_info/execution_report_2025-11-27_10-50-37.html 20
analyze sarek-dev "" \
  data/sarek/results-dev/pipeline_info/execution_report_2025-10-13_12-01-12.html 25

for route in star_salmon star_rsem; do
  # shellcheck disable=SC2046
  "$BIN" compare --top 12 $(cat examples/rnaseq-$route-releases.files) >"$OUT/rnaseq-$route-releases.md"
done

if [[ $CHECK -eq 1 ]]; then
  status=0
  for f in "$OUT"/*; do
    diff -u "examples/$(basename "$f")" "$f" || status=1
  done
  if [[ $status -eq 0 ]]; then echo "examples/: all $(find "$OUT" -type f | wc -l) generated files match"; fi
  exit $status
fi
echo "regenerated into examples/"
