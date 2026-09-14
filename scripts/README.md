# Script parameters

Edit [params.sh](params.sh), then run one of the three model scripts from the repository root.

## simulation settings

| Parameter | Meaning | Simulator option |
| --- | --- | --- |
| `USERS` | Number of independent users/runs per simulation. | `--users` |
| `HOPS` | number of nodes per path. Must match the selected topology/FPOFT preset's layer count. | `--hops` |
| `DAYS` | Simulation duration in whole days; simple and hidden service only. | `--days` |
| `CSV_INTERVAL` | time-curve interval in seconds; simple and hidden service only. Controls output resolution, not simulated events. | `--csv-interval` |
| `PLOT` | `true` generates PNG plots; `false` generates only CSVs and logs. | Script only |

## Model and sampler settings

| Parameter | Accepted values / meaning | Simulator option |
| --- | --- | --- |
| `SIMPLE_SAMPLER` | `random`, `k-hf`, `k-w`, `alpha-sticky`. | `--mode` |
| `HIDDEN_SERVICE_SAMPLER` | `fixed-path`, `fixed-topology`, `fpoft`. | `--mode` |
| `DOWNLOAD_SAMPLER` | `random`, `k-hf`, `k-w`, `alpha-sticky`. | `--mode` |
| `TOPOLOGY_PRESET` | One of the topology names below; used by `fixed-topology`. | `--topology-preset` |
| `FPOFT_PRESET` | One of the FPOFT names below; used by `fpoft`. | `--fpoft-preset` |
| `ADVERSARY` | `sybil-only`, `basic`, `apt`, `fvey`, `rubberhose1`, `rubberhose2`; hidden service only. | `--adversary` |
| `FIXED_HOPS` | Number of persistent hop positions for `k-hf`, from `0` through `HOPS`. | `--fixed-hops` |
| `K` | Positive candidate count per hop for `k-w`. Requires at least `K * HOPS` mix nodes. | `--k` |
| `ALPHA` | Probability of reusing a previous path for `alpha-sticky`, from `0` to `1`. | `--alpha` |

| Sampler | Short description |
| --- | --- |
| `random` | Samples a random path for each request. |
| `k-hf` | Keeps selected hop positions and their nodes fixed; samples the remaining hops. |
| `k-w` | Creates a persistent candidate pool per hop; chooses one node from each pool per path. |
| `alpha-sticky` | Reuses a previously selected path with probability `ALPHA`; otherwise selects a new one. |
| `fixed-path` | Keeps five paths, each independently rotating after a 1–48 hour lifetime. |
| `fixed-topology` | Samples through a local layered topology whose nodes rotate according to the preset. |
| `fpoft` | Samples from active paths over permanent topology nodes; only paths rotate. |


Time-based samplers currently work only with hidden services. Simple and download models check whether every hop of the sampled path is initially malicious.

## Fixed topology and FPOFT presets

`D2`/`D3` mean exactly 2/3 outgoing connections per non-exit node; `M` means a mesh between adjacent layers. `P5` means five distinct active paths.

| `TOPOLOGY_PRESET` | `FPOFT_PRESET` | `HOPS` |
| --- | --- | --- |
| `2_4_6_M` | `2_4_6_M_P5` | 3 |
| `2_4_8_M` | `2_4_8_M_P5` | 3 |
| `5_5_5_M` | `5_5_5_M_P5` | 3 |
| `5_5_5_D2` | `5_5_5_D2_P5` | 3 |
| `5_5_5_D3` | `5_5_5_D3_P5` | 3 |
| `5_5_5_5_M` | `5_5_5_5_M_P5` | 4 |
| `5_5_5_5_D2` | `5_5_5_5_D2_P5` | 4 |
| `5_5_5_5_D3` | `5_5_5_5_D3_P5` | 4 |

For `fixed-topology`, the `2_4_6_M` and `2_4_8_M` presets use node lifetimes of 90–120 days, 30–60 days, and 1–48 hours by layer. The five-node-layer presets use 1–48 hours in every layer.

For FPOFT, all topology nodes are permanent. Each active path has a 1–48 hour lifetime. Rotating lifetimes are sampled independently as the maximum of two uniform draws in the stated range.

## Hidden-service adversaries

| `ADVERSARY` | Per-node compromise profile |
| --- | --- |
| `sybil-only` | Uses only initially malicious nodes; no compromise events. |
| `basic` | 50% within 15 days, otherwise never. |
| `apt` | 75% within 15 days; 100% by 30 days. |
| `fvey` | 50% within 2 days; 75% within 7 days; otherwise never. |
| `rubberhose1` | 50% between 2 and 14 days inclusive, otherwise never. |
| `rubberhose2` | 50% between 7 and 21 days inclusive, otherwise never. |

## Other inputs

Network size and malicious-node fraction are Rust constants in [src/params.rs](../src/params.rs), currently 1,000 nodes and 10%. The network stays available and unchanged throughout a run. These settings are not script or CLI parameters; rebuild after editing them.
