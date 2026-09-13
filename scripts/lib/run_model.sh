#!/usr/bin/env bash
# Shared implementation for the three model scripts. Edit ../params.sh for settings.
set -euo pipefail
PROJECT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
source "$PROJECT_DIR/scripts/params.sh"
MODEL=${1:?Run run_hidden_service.sh, run_simple.sh, or run_download.sh}
case "$MODEL" in
    hidden-service) SAMPLER=$HIDDEN_SERVICE_SAMPLER ;;
    simple) SAMPLER=$SIMPLE_SAMPLER ;;
    download-session) SAMPLER=$DOWNLOAD_SAMPLER ;;
    *) echo "Unknown model: $MODEL" >&2; exit 1 ;;
esac
case "$PLOT" in
    true|false) ;;
    *) echo "PLOT in scripts/params.sh must be true or false" >&2; exit 1 ;;
esac
case "$MODEL:$SAMPLER" in
    hidden-service:fixed-path|hidden-service:fixed-topology|hidden-service:fpoft|simple:random|simple:k-hf|download-session:random|download-session:k-hf|download-session:k-w|download-session:alpha-sticky) ;;
    *) echo "Unsupported MODEL/SAMPLER: $MODEL/$SAMPLER" >&2; exit 1 ;;
esac

# Plotting is optional: CSV-only runs do not require Python or matplotlib.
if [[ $PLOT == true ]]; then
    "$PYTHON" -c 'import matplotlib' || {
        echo "Install plotting dependencies: $PYTHON -m pip install -r $PROJECT_DIR/scripts/requirements.txt" >&2
        exit 1
    }
fi
if [[ -z ${SIMULATOR_BIN:-} ]]; then
    cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml" --target-dir "$PROJECT_DIR/target"
    SIMULATOR_BIN="$PROJECT_DIR/target/release/freeroutesim"
fi
RUN_SAMPLER=$SAMPLER
if [[ $SAMPLER == fixed-topology ]]; then RUN_SAMPLER+="_${TOPOLOGY_PRESET}"; fi
if [[ $SAMPLER == fpoft ]]; then RUN_SAMPLER+="_${FPOFT_PRESET}"; fi
mkdir -p "$RESULTS_DIR"
RUN_DIR=$(mktemp -d "$RESULTS_DIR/$(date +%Y%m%d-%H%M%S)_${MODEL}_${RUN_SAMPLER}_h${HOPS}_XXXXXX")
mkdir -p "$RUN_DIR/csv" "$RUN_DIR/logs"
cp "$PROJECT_DIR/scripts/params.sh" "$RUN_DIR/params.sh"
if [[ $PLOT == true ]]; then
    mkdir -p "$RUN_DIR/plots"
    export MPLCONFIGDIR=${MPLCONFIGDIR:-"$RUN_DIR/.matplotlib"}
fi
printf '%s\n' "model=$MODEL" "sampler=$SAMPLER" "users=$USERS" "hops=$HOPS" \
    "plot=$PLOT" "simulator=$SIMULATOR_BIN" "created_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$RUN_DIR/config.txt"
if [[ $SAMPLER == fixed-topology ]]; then
    printf 'topology_preset=%s\n' "$TOPOLOGY_PRESET" >> "$RUN_DIR/config.txt"
fi
if [[ $SAMPLER == fpoft ]]; then
    printf 'fpoft_preset=%s\n' "$FPOFT_PRESET" >> "$RUN_DIR/config.txt"
fi
if revision=$(git -C "$PROJECT_DIR" rev-parse HEAD 2>/dev/null); then
    printf 'git_revision=%s\n' "$revision" >> "$RUN_DIR/config.txt"
    git -C "$PROJECT_DIR" status --short >> "$RUN_DIR/config.txt"
fi
printf '#!/usr/bin/env bash\nset -euo pipefail\n' > "$RUN_DIR/commands.sh"

args=(--model "$MODEL" --mode "$SAMPLER" --users "$USERS" --hops "$HOPS")
case "$SAMPLER" in
    fpoft) args+=(--fpoft-preset "$FPOFT_PRESET") ;;
    fixed-topology) args+=(--topology-preset "$TOPOLOGY_PRESET") ;;
    k-hf) args+=(--fixed-hops "$FIXED_HOPS") ;;
    k-w) args+=(--k "$K") ;;
    alpha-sticky) args+=(--alpha "$ALPHA") ;;
esac
if [[ $MODEL == hidden-service ]]; then args+=(--adversary "$ADVERSARY"); fi
if [[ $MODEL != download-session ]]; then args+=(--days "$DAYS" --csv-interval "$CSV_INTERVAL"); fi
csv_files=()
run_point() {
    local name=$1
    shift
    local csv_path="$RUN_DIR/csv/$name.csv"
    printf '%q ' "$SIMULATOR_BIN" "${args[@]}" "$@" --csv "$csv_path" >> "$RUN_DIR/commands.sh"
    printf '\n' >> "$RUN_DIR/commands.sh"
    "$SIMULATOR_BIN" "${args[@]}" "$@" --csv "$csv_path" 2>&1 | tee "$RUN_DIR/logs/$name.log"
    csv_files+=("$csv_path")
}
if [[ $MODEL == download-session ]]; then
    sizes=("${DOWNLOAD_SIZES[@]}")
    if [[ ${#sizes[@]} == 0 ]]; then echo "DOWNLOAD_SIZES must contain byte sizes" >&2; exit 1; fi
    seen_sizes=" "
    for size in "${sizes[@]}"; do
        if [[ ! $size =~ ^[0-9]+$ || $size =~ ^0+$ ]]; then
            echo "Invalid download byte size: $size" >&2; exit 1
        fi
        if [[ $seen_sizes == *" $size "* ]]; then
            echo "Duplicate download byte size: $size" >&2; exit 1
        fi
        seen_sizes+="$size "
    done
    for size in "${sizes[@]}"; do run_point "download_${size}_bytes" --file-size "$size" --packet-size "$PACKET_SIZE"; done
else
    run_point "$MODEL"
fi
if [[ $PLOT == true ]]; then
    "$PYTHON" "$PROJECT_DIR/scripts/plot_results.py" "${csv_files[@]}" --output-dir "$RUN_DIR/plots"
fi
printf '\nResults: %s\n' "$RUN_DIR"
