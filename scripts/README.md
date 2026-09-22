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
| `TOPOLOGY_PROFILE` | `vanguard1` (default), `vanguard2`; used by `fixed-topology`. | `--topology-profile` |
| `FPOFT_PROFILE` | `LITE`, `STANDARD` (default), `STRICT`; used by `fpoft`. | `--fpoft-profile` |
| `DOWNLOAD_PROFILE` | `LITE`, `STANDARD` (default), `STRICT`; empty for manual parameters. Requires `DOWNLOAD_SAMPLER="k-w"`. | `--download-profile` |
| `ADVERSARY` | `sybil-only`, `basic`, `apt`, `fvey`, `rubberhose1`, `rubberhose2`; hidden service only. | `--adversary` |
| `FIXED_HOPS` | Number of persistent hop positions for `k-hf`, from `0` through `HOPS`. | `--fixed-hops` |
| `KW_FIXED_HOPS` | K/W persistent pool positions, `0` through `HOPS`; empty means all hops. Remaining positions sample fresh nodes per packet. | `--fixed-hops` |
| `K` | Positive candidate count per hop for `k-w`. Requires at least `K * fixed_hops` mix nodes. | `--k` |
| `ALPHA` | Probability of reusing a previous path for `alpha-sticky`, from `0` to `1`. | `--alpha` |

| Sampler | Short description |
| --- | --- |
| `random` | Samples a random path for each request. |
| `k-hf` | Keeps selected hop positions and their nodes fixed; samples the remaining hops. |
| `k-w` | Creates a persistent candidate pool per hop; chooses one node from each pool per path. |
| `alpha-sticky` | Reuses a previously selected path with probability `ALPHA`; otherwise selects a new one. |
| `fixed-path` | Keeps five paths, each independently rotating after a 1–48 hour lifetime. |
| `fixed-topology` | Samples through a local layered topology whose nodes rotate according to the profile. |
| `fpoft` | Samples five active topology routes. Paths and nodes rotate independently. |


Time-based samplers currently work only with hidden services. Simple and download models check whether every hop of the sampled path is initially malicious.

## profiles

For hidden services, set `HIDDEN_SERVICE_SAMPLER="fpoft"` and choose `FPOFT_PROFILE` in `params.sh`, then run `bash scripts/run_hidden_service.sh`.

| `FPOFT_PROFILE` | Layer sizes, service → recipient | Connections | Permanent nodes | Rotating nodes | Possible complete routes |
| --- | --- | --- | ---: | ---: | ---: |
| `LITE` | 5–5 | Mesh | 10 | 0 | 25 |
| `STANDARD` | 5–5–R5 | Degree 3 | 10 | 5 in layer 3 | 45 |
| `STRICT` | 5–5–5–R5 | Degree 3 | 15 | 5 in layer 4 | 135 |

Each profile selects five distinct **complete routes**, including the final layer, uniformly without replacement. Each active route has its own lifetime, drawn as the maximum of two independent uniform draws from 1–48 hours. Expiry selects a route excluding those held by other active slots; the old route is eligible again.

STANDARD and STRICT independently rotate each final-layer node using the same 1–48 hour distribution. Replacements inherit the slot's connections and update every active route through that slot immediately; path timers stay unchanged. Replacements exclude all current topology nodes, including the expiring node. Retired identities may return later. Nodes in other layers never rotate. The reusable sampler also supports other per-layer lifetimes, including `Never`.

Degree 3 means three distinct **outgoing** connections per non-final-layer node. Connections prefer uncovered destinations, ensuring every topology node belongs to a possible complete route. Incoming degree can vary. An active pool of five routes need not cover every candidate. The walker observes final-layer nodes used by active routes and follows only matching active chains. Compromise records persist through rotations.

For downloads, set `DOWNLOAD_SAMPLER="k-w"` and choose `DOWNLOAD_PROFILE`, then run `bash scripts/run_download.sh`.

| `DOWNLOAD_PROFILE` | Hops | Fixed pool positions | Candidates per pool (`K`) | Fresh random positions |
| --- | ---: | ---: | ---: | ---: |
| `LITE` | 3 | 1 | 5 | 2 |
| `STANDARD` | 3 | 2 | 5 | 1 |
| `STRICT` | 4 | 3 | 3 | 1 |

These use the existing K/W sampler and S-DLM formula. Fixed positions are chosen once per session; remaining positions sample fresh nodes per packet. Downloads have no time or rotations. 

## Vanguard profiles

Set `HIDDEN_SERVICE_SAMPLER="fixed-topology"` and choose `TOPOLOGY_PROFILE`:

| Profile | Mesh layers, service → recipient | Node lifetimes by layer |
| --- | --- | --- |
| `vanguard1` | 2–4–6 | 90–120 days; 30–60 days; 1–48 hours |
| `vanguard2` | 2–4–8 | 90–120 days; 30–60 days; 1–48 hours |

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

Network size and malicious-node fraction are Rust constants in [src/params.rs](../src/params.rs), currently 1,000 nodes and 10%. The network stays available and unchanged throughout a run.

`COMPROMISE_BUDGET_PER_LAYER` in `src/params.rs` is currently `CompromiseBudget::Limited(1)`. Set `Limited(b)` for at most `b` distinct node attempts per layer, or `Unlimited` to attack all eligible discovered nodes, then rebuild.  The Sybil-only adversary always has zero attempts.
