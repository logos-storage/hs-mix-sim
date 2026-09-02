# freeroutesim

`freeroutesim` is a simulator for free-route mix. It generates topology internally and simulates independent users sending messages through mix. Users run in parallel, and every user owns a traffic model and a path sampler.

The adversary currently wins on a path only when every hop is malicious (though a model can define a different adversary win condition). Each path contains distinct mix nodes. A user's simulation stops at the adversary's first win, so the results measure both time and number of messages to first compromise. Users that are not compromised continue until the simulation time limit.

## Running the simulator

example:

```bash
cargo run --release -- \
  --mode random \
  --model simple \
  --hops 3 \
  --days 1 \
  --users 5000 \
  --epoch 3600
```

The summary is always printed. To also write results in csv (for plotting later):

```bash
cargo run -p freeroutesim --release -- \
  --mode bandwidth-random \
  --model simple \
  --hops 3 \
  --days 30 \
  --users 5000 \
  --epoch 3600 \
  --csv crates/freeroutesim/results/bandwidth_random.csv
```

View the current options with:

```bash
cargo run -p freeroutesim -- --help
```

## Command-line options

| Option | Default | Meaning |
| --- | --- | --- |
| `--mode MODE` | `random` | Path sampler: `random`, `bandwidth-random`, `guard`, `vanguard`, `k-hf`, `k-w`, or `alpha-sticky` |
| `--hops N` | `3` | Number of nodes in every path |
| `--vanguards N` | none | Number of vanguard hops; required by `vanguard` |
| `--fixed-hops N` | none | Number of persistent hop positions; required by `k-hf` |
| `--k N` | none | Candidates per logical hop; required by `k-w` |
| `--alpha P` | none | Path-reuse probability in `[0, 1]`; required by `alpha-sticky` |
| `--model MODEL` | `simple` | User model: `simple`, `hidden-service`, or `download-session` |
| `--file-size N` | none | Download size in bytes; required by `download-session` |
| `--packet-size N` | none | Total serialized Mix-packet size in bytes; required by `download-session` |
| `--days N` | `1` | Simulation duration in days; not used by `download-session` |
| `--users N` | `5000` | Number of independent users to simulate |
| `--epoch N` | `3600` | Seconds between topology updates; `download-session` uses one topology snapshot |
| `--csv PATH` | none | Write time-to-compromise and message-count results to CSV |


## Path-selection modes

All modes sample without repeating a node within one path.

### `random`

Selects every hop uniformly from the active nodes.

### `bandwidth-random`

Selects every hop in proportion to node bandwidth.

### `guard`

Places a persistent guard at hop 1. The guard is selected by bandwidth from nodes carrying the consensus-assigned `Guard` tag. All remaining hops are selected by bandwidth from the complete active topology.

Each user initially samples three guard candidates. It continues using the selected guard while that node is online. If it goes offline, the sampler selects another online node already in the user's guard set. The set is extended by one new candidate only when all candidates are offline.

### `vanguard`

Vanguard mode always includes a guard. Supply the number of vanguards with `--vanguards N`; it must be greater than zero and less than `hops - 1`, leaving at least one non-persistent hop (for the guard).

The guard remains at hop 1. Vanguards use the first available hops other than the guard hop. Four vanguards are selected by default and extended by one only when needed. Guard and vanguard candidate sets are kept separate, and the nodes selected for any one path are distinct.

All remaining hops are selected by bandwidth from the active topology.

Example using one guard, two vanguards, and one random hop:

```bash
cargo run -p freeroutesim --release -- \
  --mode vanguard \
  --hops 4 \
  --vanguards 2 \
  --model simple \
  --days 30 \
  --users 5000 \
  --epoch 3600
```

### `K-HF` (K-Hops Fixed)

Selects `--fixed-hops N` hop positions once per user and assigns one
persistent node to each selected position. Every path reuses those nodes while
sampling the remaining positions randomly. If a persistent node is offline, only that node is replaced. Nodes remain different
within each path.

`N` can range from zero through `--hops`. This mode supports the `simple` and
`download-session` models for now.

### `k-w` (K/W)

Creates a session-persistent pool of `--k K` randomly selected candidates for
each hop. Each path independently selects one node from every pool. The
pools contain different nodes, so a path cannot repeat a node, and the session can
use at most $K^L$ path combinations for path length $L$.

The topology must contain at least $K L$ active/online nodes. This mode currently
works only with `download-session`.

### `alpha-sticky` (Alpha-SS)

The first path selection adds a new path. Each later selection reuses one of
the previous paths with probability `--alpha P`, otherwise
it introduces a new path that has not appeared earlier in the session. When a
path is reused, it is selected randomly from the current set of paths.

`P` must be in `[0, 1]`. This mode currently works only with `download-session`.

## User models

### `simple`

Sends one message after each uniformly sampled interval in `[300, 900)` seconds. Every message samples a new path.

### `hidden-service`

Models bursty hidden-service traffic:

- a request arrives after each uniformly sampled interval in `[300, 900]` seconds;
- the request produces between 1 and 100 messages;
- those messages are assigned random offsets in `[0, 60]` seconds from the request time;
- every message samples its own path.

Select this model with `--model hidden-service`.

### `download-session`

Models one anonymous file download per user. It uses a single topology snapshot
and does not model elapsed time or churn.
The model derives the number of paths in the session from `--file-size`,
`--packet-size`, and `--hops`, then samples one path for every resulting packet. The formula used to compute the required number of paths/packets is based on the research document in: https://hackmd.io/@codex-storage/rJ4d1aaHfe

Example run for a 1 MiB download using three-hop K/W paths with five candidates per hop and 4608-byte Mix packets:

```bash
cargo run -p freeroutesim --release -- \
  --mode k-w \
  --k 5 \
  --model download-session \
  --hops 3 \
  --file-size 1048576 \
  --packet-size 4608 \
  --users 5000
```

## Generated topologies

The simulator creates enough topology epochs to cover the requested duration. The following defaults are constants in [`src/params.rs`](src/params.rs), but can be easily changed.

Malicious nodes are chosen randomly until we reach the node-count and bandwidth targets.

All nodes start online. At every epoch, each online node goes offline with the defined churn probability, and each offline node returns with that same probability.

### Consensus guard selection

The consensus maintains active, backup, and offline guards as follows which is similar approach to Tor:

1. Online active guards remain active; unavailable guards move offline.
2. Returning offline guards become backups.
3. If active guard bandwidth is below 25% of current online bandwidth, online backups are promoted first.
4. If backups are insufficient, new online nodes are selected by bandwidth until the target is reached.
5. Only active guards receive the `Guard` tag in the generated topology.

This consensus state is shared to all users. the tag can be extended later to add something more Tor-like e.g. fast, stable tags.

## summary

Every run prints a summary similar to:

```text
simulation_summary
users=5000
days=30
epoch_seconds=3600
topologies_loaded=721
path_hops=3
path_sampler_type=BandwidthRandomPathSampler
user_model_type=SimpleModel
malicious_node_fraction=0.100000
malicious_bandwidth_fraction=0.100000
churn_rate=0.030000
total_messages=...
users_with_compromised_messages=...
users_without_compromised_messages=...
percentage_users_compromised=...
first_compromise_timestamp_seconds=...
fewest_messages_to_first_compromise=...
```
## CSV output

When `--csv` is supplied, the simulator writes one CSV containing info for two cumulative curves. 

- `time_to_first compromise`
- `message_count_to_first compromise`

## Plotting CSV results

Install the Python dependency and run:

```bash
python3 ./scripts/plot_results.py \
  <file_name>.csv 
```

## limitations

- Guard selection is a simplified model and does not track long-term stability or online history.
- The simulator records only time and messages to first compromise.
