#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 TOPOLOGY_FILE OUT_DIR [USERS=5000] [DAYS=30] [EPOCH=3600] [ROUTESIM_FLAGS...]" >&2
  echo "Example: $0 testfiles/layout_data/bow_tie/dynamic_hybrid_steady_0.03_layout.csv results 5000 30 3600 --disable-vanguards" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CALLER_CWD="$(pwd)"

TOPOLOGY_FILE="$1"
OUT_DIR="$2"
USERS="${3:-5000}"
DAYS="${4:-30}"
EPOCH="${5:-3600}"
shift $(( $# >= 5 ? 5 : $# ))

case "$TOPOLOGY_FILE" in
  /*) ;;
  *) TOPOLOGY_FILE="$CALLER_CWD/$TOPOLOGY_FILE" ;;
esac

case "$OUT_DIR" in
  /*) ;;
  *) OUT_DIR="$CALLER_CWD/$OUT_DIR" ;;
esac

mkdir -p "$OUT_DIR"
export MPLCONFIGDIR="${MPLCONFIGDIR:-$OUT_DIR/.matplotlib}"
mkdir -p "$MPLCONFIGDIR"

RAW_OUT="$OUT_DIR/hidden_service_${USERS}_${DAYS}_${EPOCH}.routes"
OUT_PREFIX="$OUT_DIR/hidden_service_${USERS}_${DAYS}_${EPOCH}"

cd "$REPO_ROOT"
cargo run --release -- \
  --model simple-hidden-service \
  --topology-file "$TOPOLOGY_FILE" \
  --users "$USERS" \
  --days "$DAYS" \
  --epoch "$EPOCH" \
  --to-console \
  "$@" > "$RAW_OUT"

python3 "$SCRIPT_DIR/plot_v2_console.py" \
  --format hidden-service \
  --users "$USERS" \
  --in-file "$RAW_OUT" \
  --out-prefix "$OUT_PREFIX"

echo "raw_output=$RAW_OUT"
echo "per_user=$OUT_PREFIX.per_user.csv"
echo "summary=$OUT_PREFIX.summary.txt"
echo "time_plot=${OUT_PREFIX}_time_cdf.pdf"
echo "messages_plot=${OUT_PREFIX}_messages_cdf.pdf"
echo "bursts_plot=${OUT_PREFIX}_bursts_cdf.pdf"
