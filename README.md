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
| `--mode MODE` | `fixed-path` for hidden-service; `random` otherwise | Path sampler: `fixed-path`, `fixed-topology`, `fpoft`, `random`, `k-hf`, `k-w`, or `alpha-sticky` |
| `--topology-preset NAME` | `5_5_5_5_D2` | Preset for `--model hidden-service --mode fixed-topology`; see choices below |
| `--fpoft-preset NAME` | `5_5_5_5_D2_P5` | Active-path preset for `--model hidden-service --mode fpoft` |
| `--hops N` | `3` | Number of nodes in every path |
| `--fixed-hops N` | none | Number of persistent hop positions; required by `k-hf` |
| `--k N` | none | Candidates per logical hop; required by `k-w` |
| `--alpha P` | none | Path-reuse probability in `[0, 1]`; required by `alpha-sticky` |
| `--model MODEL` | `simple` | User model: `simple`, `hidden-service`, or `download-session` |
| `--adversary ADVERSARY` | `basic` for hidden-service | Hidden-service adversary: `sybil-only`, `basic`, `apt`, `fvey`, `rubberhose1`, or `rubberhose2`; rejected for other models |
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

### `fixed-topology`

A time-based hidden-service sampler that builds a separate local layered topology
for each user. It stores nodes rather than a pool of complete paths. Layer 1 is
nearest the service/sender; the last layer contains the recipient-side exits.

`FixedTopology` contains the nodes, connections, lifetimes, events, and observation
logic. `FixedTopologySampler` adapts it to the existing sampler traits. Other samplers
can reuse the struct or construct it using another slice of `LayerConfig` values.

Each layer is configured with `LayerConfig::new(node_count, lifetime)`. The
`NodeLifetime` policy applies to every node in that layer: `NodeLifetime::Never`
keeps nodes permanently; `NodeLifetime::MaxOfTwoUniform { min_seconds, max_seconds }`
uses an inclusive lifetime range in seconds.
Initialization samples distinct mixes across all layers from the global mixnet.
Rotating nodes independently draw their lifetime as the maximum of two uniform
draws inside their layer's range. Permanent nodes have no expiration timestamp and
never generate rotation events; they remain available for sampling and observation. At expiration, only that slot is replaced, with a fresh
lifetime from the same layer. The replacement is drawn uniformly from nodes outside
the current local topology, including excluding the expiring node. Consequently,
the network needs at least one spare mix beyond the sum of layer sizes if any
layer rotates. An entirely permanent topology needs no spare nodes. Previously
removed mixes may return at later rotations; adversary attempt records persist.

Each experiment defines its connection mode as `ConnectionMode::Degree(d)` or
`ConnectionMode::Mesh` in its preset.

`Degree(d)` gives every non-exit node exactly `d` distinct outgoing neighbors in
its next layer. For each adjacent pair with `a` source nodes and `b` destination
nodes, the configuration must satisfy `d > 0`, `d <= b`, and `a * d >= b`.
Impossible configurations are rejected before drawing connections.

Connection generation shuffles source and destination slots, then preferentially
assigns destinations that have no incoming connection yet. Once all destinations
are covered, remaining links are sampled randomly without duplicates within a
source's neighbor list. This guarantees incoming coverage, although incoming degrees
can differ. It is a coverage-first construction, not uniform sampling over all
possible graphs. Every selected node belongs to at least one complete path.

`Mesh` connects every node to **every node in the next layer**. It works with
unequal layer sizes and needs no degree setting. It adds no same-layer links or
links that skip layers. The last layer has no outgoing links in either mode.

Connections are chosen once between slots; rotating a node preserves its slot's
connections and coverage. There is no script or CLI connection-mode parameter.
Different sampler implementations can supply their own mode when constructing
`FixedTopology`.

Path requests choose a first-layer node uniformly, then an outgoing neighbor
uniformly at each step. Both modes give equal probability to each valid complete
path. `peak` validates the whole reverse chain against those connections before
revealing adjacent neighbors. `FixedTopology::paths()` optionally enumerates the
unique valid path set `P`. Its size is `first_layer_size * d^(hops - 1)` for
`Degree(d)`, or the product of layer sizes for `Mesh`. Normal sampling and observations
avoid materializing this potentially large set.

Experiments are named constants in
`src/time_based_path_sampler/fixed_topology/presets.rs`. They share one sampler
implementation; only the layer configuration and connection mode differ.

| Preset | Layers (service → exits) | Lifetimes | Connections |
| --- | --- | --- | --- |
| `2_4_6_M` | 2, 4, 6 | 90–120 days; 30–60 days; 1–48 hours | Mesh |
| `2_4_8_M` | 2, 4, 8 | 90–120 days; 30–60 days; 1–48 hours | Mesh |
| `5_5_5_M` | 5, 5, 5 | 1–48 hours in every layer | Mesh |
| `5_5_5_D2` | 5, 5, 5 | 1–48 hours in every layer | Degree 2 |
| `5_5_5_D3` | 5, 5, 5 | 1–48 hours in every layer | Degree 3 |
| `5_5_5_5_M` | 5, 5, 5, 5 | 1–48 hours in every layer | Mesh |
| `5_5_5_5_D2` | 5, 5, 5, 5 | 1–48 hours in every layer | Degree 2 |
| `5_5_5_5_D3` | 5, 5, 5, 5 | 1–48 hours in every layer | Degree 3 |

Names list the **number of nodes in each hop**, from service to exits.
`_D2` means degree 2, `_D3` means degree 3, and `_M` means mesh.
All rotating lifetimes use the maximum of two independent uniform draws.
Select an existing preset using `--topology-preset`, without rebuilding. For example,
`--topology-preset 5_5_5_D2 --hops 3` selects three layers of five nodes at degree 2.

The default is `5_5_5_5_D2`. `--hops` / `HOPS` must match the selected preset's
layer count. Unknown names, use with another model or sampler, and mismatched hop
counts are rejected. The selected name is recorded as the sampler name in
summaries, CSVs, and plots.

To add an experiment, define another `TopologyExperiment` constant in `presets.rs`
and include it in `ALL_EXPERIMENTS`. Rebuild once to expose it in the CLI choices
and shared validation tests. Rust constant names have a `TOPOLOGY_` prefix because
identifiers cannot begin with a digit:

```rust
pub const TOPOLOGY_2_4_3_M: TopologyExperiment = TopologyExperiment {
    name: "2_4_3_M",
    layers: &[
        LayerConfig::new(2, NodeLifetime::Never),
        LayerConfig::new(4, NodeLifetime::MaxOfTwoUniform {
            min_seconds: 6 * 3600,
            max_seconds: 24 * 3600,
        }),
        LayerConfig::new(3, SHORT_ROTATION),
    ],
    connections: ConnectionMode::Mesh,
};
```

No sampler implementation or macro is needed for a new configuration.
`FixedTopologySampler::new(experiment, &mixnet)` constructs it programmatically.
For direct access to the optional path set `P`, construct
`FixedTopology::new(experiment.layers, experiment.connections, &mixnet)` and call
`paths()` on it.

Permanent layers produce no sampler events; adversary compromise events still run.
If every layer is permanent and the adversary schedules nothing, the model stops
after its initial check.

To use it with the hidden-service script, edit `scripts/params.sh`:

```bash
HIDDEN_SERVICE_SAMPLER="fixed-topology"
TOPOLOGY_PRESET="5_5_5_5_D2"
HOPS=4
```

Then run `bash scripts/run_hidden_service.sh`. Or run the simulator directly:

```bash
cargo run --release -- --model hidden-service --mode fixed-topology \
  --topology-preset 5_5_5_5_D2 --hops 4 --adversary basic --days 30 --users 5000
```

The CSV and plot caption include per-layer sizes, lifetime ranges, connection mode,
degree when applicable, and
lifetime distribution. Permanent layers have `never` in both lifetime-bound CSV
fields and are labeled “never expires” in plots. The global mixnet remains static; only local membership
rotates. `fixed-path` remains the default sampler for hidden services.

### `fpoft` — fixed paths over a fixed topology

This sampler builds a local topology and enumerates its possible routes once.
**Topology nodes and connections remain permanent.** It reuses a topology preset's
layer sizes and connection mode, and disables all node lifetimes from that preset.
Only the active paths have lifetimes in this mode.

At initialization, it selects `num_paths` distinct routes uniformly without
replacement. Each active slot independently samples its own expiry using
`Lifetime::MaxOfTwoUniform { min_seconds, max_seconds }`, or has no expiry with
`Lifetime::Never`. Requests choose uniformly from the active pool. `peak` reveals
only chains belonging to currently active paths; inactive topology routes cannot
identify the service.

An expiry resamples just that slot, excluding routes used by the other active
slots, and independently samples a fresh lifetime. The expired route can be
selected again. This also permits a pool containing every possible route.
The pool size must be positive and cannot exceed the number of topology routes.
Node identities never change, and adversary compromise records persist.
If paths never expire, there are no sampler events, but adversary events still run.

FPOFT definitions are `FPOFTExperiment` constants in
`src/time_based_path_sampler/fixed_topology/presets.rs`. They contain a name, a
reference to a topology configuration, the active-path count, and the path lifetime.
The eight topology names each have a `_P5` variant, for example `2_4_6_M_P5` and
`5_5_5_D2_P5`, with five active paths and independent 1–48 hour lifetimes.
`5_5_5_D2_P5_NEVER` demonstrates permanent active paths. Add a constant to
`ALL_FPOFT_EXPERIMENTS` and rebuild to expose another count or lifetime in the CLI.
`Lifetime` is shared by nodes and paths; `NodeLifetime` remains a compatible name
for existing topology configuration code.

```bash
cargo run --release -- --model hidden-service --mode fpoft \
  --fpoft-preset 2_4_6_M_P5 --hops 3 --adversary basic --days 30 --users 5000
```

Or edit `scripts/params.sh` and run `bash scripts/run_hidden_service.sh`:

```bash
HIDDEN_SERVICE_SAMPLER="fpoft"
FPOFT_PRESET="2_4_6_M_P5"
HOPS=3
```

Use `--fpoft-preset` with `fpoft`; `--topology-preset` applies to the direct
`fixed-topology` sampler. CSVs and plot captions include the selected FPOFT name,
active-path count and lifetimes, and the effective permanent-node topology.
This implementation enumerates all topology routes, so memory grows with the
number of possible routes, not only the active-path count.

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

- initializes the time-based sampler at time 0, queues its rotation events, and walks the initial paths;
- queues successful compromise attempts and retains failed attempts permanently;
- jumps directly to the earliest scheduled event, with insertion order breaking timestamp ties;
- delivers each event to its sampler or adversary, then walks again from the current exits;
- processes events until the adversary wins, the queue is empty, or the next event exceeds the inclusive simulation deadline;
- checks the deadline before processing the next event.

All six hidden-service adversaries use the same persistent walker and initial
Sybil control. Select one using `--adversary` or `ADVERSARY` in `scripts/params.sh`:

| CLI value | Compromise outcome after first discovery of an honest node |
| --- | --- |
| `sybil-only` | No compromise attempts succeed; only initially malicious nodes are controlled |
| `basic` (default) | 50% within 15 days, otherwise never |
| `apt` (also `APT`) | 75% within 15 days; 100% by 30 days |
| `fvey` (also `FVEY`) | 50% within 2 days; 75% within 7 days; otherwise never |
| `rubberhose1` | 50% between 2 and 14 days inclusive, otherwise never |
| `rubberhose2` | 50% between 7 and 21 days inclusive, otherwise never |

Milestone percentages are cumulative. APT assigns 75% of attempts to [1 second,
15 days] and 25% to [15 days + 1 second, 30 days]. FVEY assigns 50% to [1 second,
2 days], 25% to [2 days + 1 second, 7 days], and 25% to permanent failure.
Successful completion times are uniform in the selected interval. Rubberhose
profiles assign no probability before their minimum day, and include both endpoints.
All times are delays from the node's first discovery, not from simulation time zero.

Each honest node's outcome is sampled once. Rediscovery never retries or resets
an attempt. Malicious nodes are controlled immediately without an attempt record.
Pending and completed attempts survive path rotation, even when the node is no
longer visible. These profiles work with `fixed-path`, `fixed-topology`, and `fpoft`.
Simple and download-session models retain their per-path Sybil adversary.

CSV metadata records success intervals using semicolon-separated
`node_compromise_interval_min_seconds`, `node_compromise_interval_max_seconds`,
and `node_compromise_interval_probabilities`. These probabilities are interval
masses, not cumulative values. `node_compromise_probability` is the total success
probability; `node_compromise_never_probability` is its complement. Plot captions
report the full profile, and the plot script still supports earlier CSVs.

Every win records the number of completed node compromises, including completed
attempts on paths that have since rotated away, and excluding initial Sybil control.
Console summaries report totals and a mean over winning users. The CSV
`mean_node_compromises_before_win` column reports the mean among users identified
by each timestamp; it is empty if nobody has won yet. Hidden-service CSVs contain
only the time curve, because event checks do not represent traffic.

Select this model with `--model hidden-service`; `fixed-path` is the default,
and `fixed-topology` is also available:

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
Use `HIDDEN_SERVICE_SAMPLER` to choose `fixed-path`, `fixed-topology`, or `fpoft`;
`SIMPLE_SAMPLER` and `DOWNLOAD_SAMPLER` select the other models' samplers independently.
For FPOFT, set `FPOFT_PRESET` in `scripts/params.sh` and match `HOPS` to its layer count.
For a local topology sampled directly, set `TOPOLOGY_PRESET` in `scripts/params.sh` and match `HOPS`
to its layer count. The result folder and saved configuration include the preset
name. Edit or add preset definitions in
`src/time_based_path_sampler/fixed_topology/presets.rs`.

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
