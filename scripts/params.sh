#!/usr/bin/env bash
# Edit this file, then run one of the three model scripts.
# Full parameter reference: scripts/README.md

# Shared settings.
USERS=5000
HOPS=4                         # ignored when a profile sets the hop count
DAYS=30                         # simple and hidden service only
CSV_INTERVAL=3600               # time-curve reporting resolution, seconds
PLOT=true                       # true: generate plots; false: CSVs and logs only

# Hidden service. Profiles determine their own hop counts.
HIDDEN_SERVICE_SAMPLER="fpoft"  # fixed-path, fixed-topology, or fpoft
TOPOLOGY_PROFILE="vanguard1"     # vanguard1 or vanguard2; fixed-topology only
# LITE: 2 mesh layers; STANDARD: 3 layers, degree 3; STRICT: 4 layers, degree 3.
# All keep five complete active paths. STANDARD/STRICT rotate final-layer nodes.
FPOFT_PROFILE="STANDARD"         # LITE, STANDARD, or STRICT
ADVERSARY="sybil-only"              # sybil-only, basic, apt, fvey, rubberhose1, rubberhose2

# Simple model.
SIMPLE_SAMPLER="random"        # random, k-hf, k-w, or alpha-sticky

# Download model.
DOWNLOAD_SAMPLER="k-w"          # random, k-hf, k-w, or alpha-sticky
DOWNLOAD_PROFILE="STANDARD"      # LITE, STANDARD, STRICT; empty for manual settings
# Download profiles set HOPS, K, and KW_FIXED_HOPS; require DOWNLOAD_SAMPLER="k-w".
PACKET_SIZE=4608                # serialized Mix packet, bytes
DOWNLOAD_SIZES=(65536 262144 1048576 4194304 16777216) # file sizes, bytes

# Sampler parameters; used only when that sampler is selected.
FIXED_HOPS=2                   # k-hf: persistent hop positions
KW_FIXED_HOPS=""               # k-w: empty = all hops, or 0..HOPS
K=5                            # k-w: candidates per hop
ALPHA=0.95                     # alpha-sticky: path-reuse probability

# Output and tools. PROJECT_DIR is set by the runner before loading this file.
RESULTS_DIR="$PROJECT_DIR/results"
PYTHON="python3"               # used only when PLOT=true
SIMULATOR_BIN=""               # empty: build release; or set a prebuilt binary path
