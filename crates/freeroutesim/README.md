# freeroutesim

`freeroutesim` is a simulator for free-route mix. It generates topology internally and simulates independent users sending messages through mix. Users run in parallel, and every user model owns a path sampler and an adversary.

The simple and download-session models use a Sybil adversary that wins when every hop on a sampled path is malicious. The hidden-service model uses a persistent adversary that discovers nodes from the exits toward the service, compromises honest nodes over time, and wins when it can walk a controlled path to the service. Each path contains distinct mix nodes. A user's simulation stops at the adversary's first win, an empty event queue, or the simulation time limit.

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
| `--mode MODE` | `fixed-path` for hidden-service; `random` otherwise | Path sampler: `fixed-path`, `random`, `bandwidth-random`, `k-hf`, `k-w`, or `alpha-sticky` |
| `--hops N` | `3` | Number of nodes in every path |
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

### `fixed-path`

The time-based sampler used by `hidden-service`. Each user stores five independently
sampled paths. Each path expires after the maximum of two independent uniform draws
between 1 and 48 hours, with second-level resolution. A rotation replaces that path
using the topology at the event's timestamp and schedules its next expiration.
Path requests choose uniformly from the five stored paths. Rotation does not clear
the adversary's knowledge or pending compromises.

### `random`

Selects every hop uniformly from the active nodes.

### `bandwidth-random`

Selects every hop in proportion to node bandwidth.

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

Uses a synchronous event queue with simulated time in `u64` seconds:

- initializes the fixed-path sampler at time 0, queues its rotation events, and walks the initial paths;
- queues successful compromise attempts and retains failed attempts permanently;
- jumps directly to the earliest scheduled event, with insertion order breaking timestamp ties;
- delivers each event to its sampler or adversary, then walks again from the current exits;
- processes events until the adversary wins, the queue is empty, or the next event exceeds the inclusive simulation deadline;
- checks the deadline before processing the next event.

The basic adversary samples each honest node's outcome once: a 50% chance of
completion uniformly between 1 second and 15 days after discovery, otherwise
the node can never be compromised. Rediscovery never retries or resets an attempt.
Malicious nodes are controlled immediately without an attempt record. Pending and
completed attempts survive path rotation, even when their node is no longer visible.

The shared adversary logic supports cumulative probability milestones. For example,
50% by day 7 and 75% by day 14 assigns 50% of attempts to days 0–7, another 25% to
days 7–14, and the remaining 25% to permanent failure. Completion times are uniform
within the selected interval.

Every win records the number of completed node compromises, including completed
attempts on paths that have since rotated away, and excluding initial Sybil control.
Console summaries report totals and a mean over winning users. CSV columns
`cumulative_node_compromises_before_win`, `wins_with_compromise_counts`, and
`mean_node_compromises_before_win` track these counts over time. Existing CSV
message-index fields count win checks for this event-driven model.

Select this model with `--model hidden-service`; `fixed-path` is its default and
currently its only supported sampler:

```bash
cargo run -p freeroutesim --release -- \
  --model hidden-service --mode fixed-path --hops 3 --days 30 --users 5000
```

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

- The simulator records only time and messages to first compromise.
