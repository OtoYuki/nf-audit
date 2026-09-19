#!/usr/bin/env bash
# Pull public nf-core AWS megatest execution traces and reports (no credentials needed).
#
# Usage: scripts/pull-megatests.sh [pipeline] [results-prefix]
#   scripts/pull-megatests.sh rnaseq results-4053b2ec173fc0cfdd13ff2b51cfaceb59f783cb   # rnaseq 3.15.1 test_full
#   scripts/pull-megatests.sh rnaseq results-0bb032c1e3b1e1ff0b0a72192b9118fdb5062489   # rnaseq 3.19.0 test_full
#   scripts/pull-megatests.sh sarek results-dev
# List available result prefixes for a pipeline:
#   scripts/pull-megatests.sh --list rnaseq
#
# Layout differs by release: newer runs write `results-<sha>/pipeline_info/`, older rnaseq runs
# write one `pipeline_info/` per aligner (`results-<sha>/aligner_star_salmon/pipeline_info/`).
# Every `pipeline_info/` under the prefix is fetched, preserving the sub-path under data/.
set -euo pipefail
BUCKET="https://nf-core-awsmegatests.s3.eu-west-1.amazonaws.com"

list_keys() { # every key under a prefix, following continuation tokens
  local prefix="$1" token="" page
  while :; do
    if [[ -n "$token" ]]; then
      page=$(curl -sS --retry 3 "$BUCKET/?list-type=2&prefix=$prefix&max-keys=1000&continuation-token=$(python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1],safe=""))' "$token")")
    else
      page=$(curl -sS --retry 3 "$BUCKET/?list-type=2&prefix=$prefix&max-keys=1000")
    fi
    printf '%s' "$page" | grep -o '<Key>[^<]*</Key>' | sed 's/<[^>]*>//g'
    token=$(printf '%s' "$page" | grep -o '<NextContinuationToken>[^<]*</NextContinuationToken>' | sed 's/<[^>]*>//g' | python3 -c 'import sys,html;print(html.unescape(sys.stdin.read().strip()))')
    [[ -z "$token" ]] && break
  done
}

if [[ "${1:-}" == "--list" ]]; then
  curl -sS "$BUCKET/?list-type=2&prefix=${2:?pipeline}/&delimiter=/" | grep -o '<Prefix>[^<]*</Prefix>' | sed 's/<[^>]*>//g'
  exit 0
fi
PIPE="${1:?pipeline, e.g. rnaseq}"
PREFIX="${2:?results prefix, e.g. results-<sha> or results-dev}"
OUT="data/$PIPE/$PREFIX"
n=0
while read -r k; do
  case "$k" in
    *execution_trace_*.txt|*execution_report_*.html|*params_*.json|*pipeline_dag_*.html)
      rel="${k#"$PIPE/$PREFIX/"}"
      f="$OUT/$rel"
      mkdir -p "$(dirname "$f")"
      if [[ ! -s "$f" ]]; then echo "get $k"; curl -sS --retry 3 -o "$f" "$BUCKET/$k"; fi
      n=$((n+1)) ;;
  esac
done < <(list_keys "$PIPE/$PREFIX/" | grep '/pipeline_info/')
if [[ $n -eq 0 ]]; then echo "no pipeline_info/ found under $PIPE/$PREFIX/"; exit 1; fi
find "$OUT" -type f | sort
echo
echo "Next: cargo run --release -- inspect <one of the execution_report_*.html above>"
