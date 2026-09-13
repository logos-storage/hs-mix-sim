#!/usr/bin/env bash
# Edit this file, then run one of the three model scripts.

# Shared settings.
USERS=5000
HOPS=4
DAYS=30                         # simple and hidden service only
CSV_INTERVAL=3600               # time-curve reporting resolution, seconds
PLOT=true                       # true: generate plots; false: CSVs and logs only

# Hidden service (always uses the fixed-path sampler).
ADVERSARY="basic"              # basic or sybil-only

# Simple model.
SIMPLE_SAMPLER="random"        # random or k-hf

# Download model.
DOWNLOAD_SAMPLER="random"      # random, k-hf, k-w, or alpha-sticky
PACKET_SIZE=4608                # serialized Mix packet, bytes
DOWNLOAD_SIZES=(65536 262144 1048576 4194304 16777216) # file sizes, bytes

# Sampler parameters; used only when that sampler is selected.
FIXED_HOPS=2                   # k-hf: persistent hop positions
K=5                            # k-w: candidates per hop
ALPHA=0.95                     # alpha-sticky: path-reuse probability

# Output and tools. PROJECT_DIR is set by the runner before loading this file.
RESULTS_DIR="$PROJECT_DIR/results"
PYTHON="python3"               # used only when PLOT=true
SIMULATOR_BIN=""               # empty: build release; or set a prebuilt binary path
