# freeroutesim

`freeroutesim` is a simulator for free-route mix. It generates one static mixnet shared by all users in a run and simulates independent users sending messages through mix. Users run in parallel, and every user model owns a path sampler and an adversary.

The simple and download-session models use a Sybil adversary that wins when every hop on a sampled path is malicious. The hidden-service model uses a configurable persistent adversary that discovers nodes from the exits toward the service, may compromise honest nodes over time, and wins when it can walk a controlled path to the service. Each path contains distinct mix nodes. A user's simulation stops at the adversary's first win, an empty event queue, or the simulation time limit.

## Running the simulator

example:

```bash
cargo run --release -- \
  --mode random \
  --model simple \
  --hops 3 \
  --days 1 \
  --users 5000
```

The summary is always printed. To also write results in csv (for plotting later):

```bash
cargo run --release -- \
  --mode random \
  --model simple \
  --hops 3 \
  --days 30 \
  --users 5000 \
  --csv results/random.csv
```

View the current options with:

```bash
cargo run -- --help
```

## Command-line options

| Option | Default | Meaning |
| --- | --- | --- |
| `--mode MODE` | `fixed-path` for hidden-service; `random` otherwise | Path sampler: `fixed-path`, `random`, `k-hf`, `k-w`, or `alpha-sticky` |
| `--hops N` | `3` | Number of nodes in every path |
| `--fixed-hops N` | none | Number of persistent hop positions; required by `k-hf` |
| `--k N` | none | Candidates per logical hop; required by `k-w` |
| `--alpha P` | none | Path-reuse probability in `[0, 1]`; required by `alpha-sticky` |
| `--model MODEL` | `simple` | User model: `simple`, `hidden-service`, or `download-session` |
| `--adversary ADVERSARY` | `basic` for hidden-service | Hidden-service adversary: `basic` or `sybil-only`; rejected for other models |
| `--file-size N` | none | Download size in bytes; required by `download-session` |
| `--packet-size N` | none | Total serialized Mix-packet size in bytes; required by `download-session` |
| `--days N` | `1` | Simulation duration in days; not used by `download-session` |
| `--users N` | `5000` | Number of independent users to simulate |
| `--csv-interval N` | `3600` | Seconds between CSV time-series rows; affects output resolution only |
| `--csv PATH` | none | Write model-specific results and run metadata to CSV |


## Path-selection modes

All modes sample without repeating a node within one path.

### `fixed-path`

The time-based sampler used by `hidden-service`. Each user stores five independently
sampled paths. Each path expires after the maximum of two independent uniform draws
between 1 and 48 hours, with second-level resolution. A rotation replaces that path
using the same static mixnet and schedules its next expiration.
Path requests choose uniformly from the five stored paths. Rotation does not clear
the adversary's knowledge or pending compromises.

### `random`

Selects every hop uniformly from all mix nodes.

### `K-HF` (K-Hops Fixed)

Selects `--fixed-hops N` hop positions once per user and assigns one
persistent node to each selected position. Every path reuses those nodes while
sampling the remaining positions randomly. The fixed nodes stay selected for the
entire run. Nodes remain different within each path.

`N` can range from zero through `--hops`. This mode supports the `simple` and
`download-session` models for now.

### `k-w` (K/W)

Creates a session-persistent pool of `--k K` randomly selected candidates for
each hop. Each path independently selects one node from every pool. The
pools contain different nodes, so a path cannot repeat a node, and the session can
use at most $K^L$ path combinations for path length $L$.

The mixnet must contain at least $K L$ mix nodes. This mode currently
works only with `download-session`.

### `alpha-sticky` (Alpha-SS)

The first path selection adds a new path. Each later selection reuses one of
the previous paths with probability `--alpha P`, otherwise
it introduces a new path that has not appeared earlier in the session. When a
path is reused, it is selected randomly from the current set of paths.

`P` must be in `[0, 1]`. This mode currently works only with `download-session`.

## User models

### `simple`

Sends one message after each uniformly sampled interval in `[300, 900)` seconds. Every message samples a new path from the static network. Stops before sampling a message beyond the inclusive simulation deadline.

### `hidden-service`

Uses a synchronous event queue with simulated time in `u64` seconds:

- initializes the fixed-path sampler at time 0, queues its rotation events, and walks the initial paths;
- queues successful compromise attempts and retains failed attempts permanently;
- jumps directly to the earliest scheduled event, with insertion order breaking timestamp ties;
- delivers each event to its sampler or adversary, then walks again from the current exits;
- processes events until the adversary wins, the queue is empty, or the next event exceeds the inclusive simulation deadline;
- checks the deadline before processing the next event.

The default `--adversary basic` samples each honest node's outcome once: a 50% chance of
completion uniformly between 1 second and 15 days after discovery, otherwise
the node can never be compromised. Rediscovery never retries or resets an attempt.
Malicious nodes are controlled immediately without an attempt record. Pending and
completed attempts survive path rotation, even when their node is no longer visible.

With `--adversary sybil-only`, the walker uses only initially malicious mixes.
Honest nodes can never be compromised, so it schedules no compromise events;
path rotation events still trigger new win checks. This option applies only to
`hidden-service`. Simple and download-session models use their existing per-path
Sybil adversary. Console summaries identify the implementation as `adversary_type`.

The shared adversary logic supports cumulative probability milestones. For example,
50% by day 7 and 75% by day 14 assigns 50% of attempts to days 0–7, another 25% to
days 7–14, and the remaining 25% to permanent failure. Completion times are uniform
within the selected interval.

Every win records the number of completed node compromises, including completed
attempts on paths that have since rotated away, and excluding initial Sybil control.
Console summaries report totals and a mean over winning users. The CSV
`mean_node_compromises_before_win` column reports the mean among users identified
by each timestamp; it is empty if nobody has won yet. Hidden-service CSVs contain
only the time curve, because event checks do not represent traffic.

Select this model with `--model hidden-service`; `fixed-path` is its default and
currently its only supported sampler:

```bash
cargo run --release -- \
  --model hidden-service --mode fixed-path --hops 3 --days 30 --users 5000
```

To use only initial Sybil control with four-hop fixed paths:

```bash
cargo run --release -- \
  --model hidden-service --adversary sybil-only --hops 4 --days 30 --users 5000
```

### `download-session`

Models one anonymous file download per user using the shared static mixnet.
It does not model elapsed time.
The model derives the number of paths in the session from `--file-size`,
`--packet-size`, and `--hops`, then samples one path for every resulting packet. The formula used to compute the required number of paths/packets is based on the research document in: https://hackmd.io/@codex-storage/rJ4d1aaHfe

Example run for a 1 MiB download using three-hop K/W paths with five candidates per hop and 4608-byte Mix packets:

```bash
cargo run --release -- \
  --mode k-w \
  --k 5 \
  --model download-session \
  --hops 3 \
  --file-size 1048576 \
  --packet-size 4608 \
  --users 5000
```

## Static mixnet

The simulator generates one network per run and shares it across all users. Its
node membership and initial malicious flags remain fixed
throughout the run, regardless of `--days`. The defaults are constants in
[`src/params.rs`](src/params.rs).

The generator uniformly selects `ceil(mix_size * malicious_node_fraction)` distinct
malicious nodes. With 1,000 nodes and a 10% target, exactly 100 are malicious.
All samplers choose nodes uniformly from their eligible candidates.

All mix nodes are always available. The mixnet stays fixed throughout the run, without churn.
Hidden-service path rotations and adversary compromise events still advance
simulated time and operate on the same network. Acquired compromises remain in
the adversary state; they do not mutate the shared mixnet.

### Controlled subsets

`mixnet.sample_subset(size, malicious_fraction)` returns randomly selected existing
nodes without duplicates. For example, `sample_subset(100, 0.02)` requests 100 nodes
with 2 malicious and 98 honest nodes. Fractions use the range `[0, 1]`; fractional
node counts round down so the result never exceeds the requested fraction.

The requested fraction must not exceed the mixnet's actual malicious node fraction.
Selection is uniform within the honest and malicious groups, and the result is shuffled.
Node IDs and malicious flags are preserved. Requests fail if the size is too large
or there are not enough nodes to meet the requested composition. An empty mixnet
allows only an empty subset with fraction zero.

This helper models the assumed quality of a selected set; it does not mean a real
user can identify malicious nodes. It is available for future samplers. Existing
samplers continue to use the full mixnet unless supplied a mixnet built from a subset.

## Summary and CSV output

The console reports configuration, compromised and uncompromised users, and the
percentage compromised. Simple and hidden-service summaries also report the first
compromise time. Hidden services include completed-node compromise statistics;
downloads include the formula S-DLM estimate. Reporting interval, total processed
messages/checks, and fewest messages/checks to a win are omitted.

CSV probabilities are in `[0, 1]`. Plots display them as percentages. The denominator
is **all simulated users**, including runs that never win. Each user contributes
only its first compromise. These are empirical results within the configured run
limits, not estimates of eventual compromise beyond those limits.

Every row includes `model`, `sampler`, `hops`, `adversary`, `users`, `mix_nodes`,
`malicious_nodes`, and the **actual** `malicious_node_fraction`. Relevant sampler
parameters (`fixed_hops`, `k`, or `alpha`) are included when used. Hidden-service
metadata includes stored-path count, lifetime distribution and bounds, and the
adversary's compromise probability and conditional delay profile. Simple metadata
includes message-interval bounds. Download metadata includes serialized packet size.

| Model | Plot data columns | Generated plots |
| --- | --- | --- |
| Simple | `curve`, `x`, `compromised_users`, `cumulative_probability` | Probability vs messages; probability vs time |
| Hidden service | `curve`, `x`, `compromised_users`, `cumulative_probability`, `mean_node_compromises_before_win` | Probability vs time only |
| Download | `download_size_bytes`, `packet_count`, `compromised_users`, `simulated_s_dlm`, `formula_s_dlm` | Simulated and formula S-DLM vs download size |

For time curves, `curve=time_seconds` and `x` is seconds from initialization.
Rows include time zero and the inclusive deadline, with hourly reporting in between
by default. `--csv-interval N` controls reporting resolution only; it does not advance
simulation time. Wins are counted at the first reporting timestamp at or after the
win, so sub-interval timing is not visible in these plots. For simple message curves,
`curve=message_count` and `x` is a **per-user** message count. Rows include zero,
every count at which users first win, and the largest observed count. The message
curve is limited by each user's simulation deadline and keeps all users in its
denominator; it does not extrapolate extra messages for users whose time ran out.

Downloads export one row for the requested file size. `packet_count` is the full
session's required number of packets/paths, including erasure coding, SURB supply,
and control overhead, even if simulation stops early at a win. It is not the number
of packets processed before compromise. Downloads have no time curve. The existing
S-DLM formulas remain approximations: they use powers of the malicious fraction,
while simulation selects distinct nodes within paths. K/W also approximates path
exposure using the capped number of possible combinations. A mismatch between the
formula and simulation is possible, especially for persistent candidate pools.

## Run simulations and generate plots

Edit **`scripts/params.sh`** to set the run count, hops, duration, samplers,
adversary, and download sizes. Then run the script for your model:

```bash
bash scripts/run_hidden_service.sh
bash scripts/run_simple.sh
bash scripts/run_download.sh
```

All three scripts read the same settings file. No environment-variable commands
are needed. For example, set `ADVERSARY="sybil-only"` for Sybil-only hidden services,
or `DOWNLOAD_SAMPLER="k-w"` and `K=5` for a download sweep using K/W.
Hidden services always use fixed paths; `SIMPLE_SAMPLER` and `DOWNLOAD_SAMPLER`
select the other models' samplers independently.

In `scripts/params.sh`, set:

```bash
PLOT=true   # generate CSVs, logs, and plots
# or
PLOT=false  # generate CSVs and logs only; no Python/matplotlib required
```

For plotting, install the dependency once:

```bash
python3 -m pip install -r scripts/requirements.txt
```

Defaults are 5,000 runs, four hops, and 30 days for time-based models. Hidden
services use the basic adversary; simple and download models use random sampling.
Mixnet size and malicious fraction remain Rust constants in `src/params.rs`
(1,000 nodes and 10% by default).

Each script builds the release binary, runs its model, and generates plots if
`PLOT=true`. Downloads sweep the `DOWNLOAD_SIZES` array of byte sizes; each size
is an independent simulation with a fresh static mixnet and fresh users. There
is no seeded replay option currently. Advanced settings in the same file select
the output directory, Python interpreter, or a prebuilt binary to skip building.

Each invocation creates a unique directory, preserving previous results:

```text
results/<timestamp>_<model>_<sampler>_h<hops>_<unique>/
  params.sh        # snapshot of the shared settings used
  config.txt       # run identity, binary path, Git revision/status when available
  commands.sh      # exact simulator commands and arguments used
  csv/             # one CSV per run/download size, including full run metadata
  logs/            # console summary for each simulation
  plots/           # PNG plots with configuration captions; only when PLOT=true
```

The three model scripts share their implementation in `scripts/lib/run_model.sh`;
you only need to edit `scripts/params.sh`. They replace `run_simulation.sh` and
the older `evaluate_download_samplers.py` workflow.

You can also plot exported CSVs directly:

```bash
python3 scripts/plot_results.py results/random.csv --output-dir results/plots
python3 scripts/plot_results.py results/<run>/csv/*.csv --output-dir results/<run>/plots
```

The plotter reads labels and configuration from the CSVs. It writes separate time
and message plots for simple runs, and a time plot for hidden services. Download
points with matching configuration are grouped into one plot, with both S-DLM
curves and tick labels such as `1 MiB [481 packets]` (using actual exported counts).
The download-size axis is logarithmic. Different configurations get separate plots;
one CSV per size/configuration is expected. Old generic time-series CSVs must be
regenerated with the new exporter.

For verification:

```bash
cargo test
python3 -m unittest discover -s scripts -p 'test_*.py'
```
