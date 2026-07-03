# hs-mix-sim

`hs-mix-sim` is a simplified simulator for route-compromise simulations. It is based on [routesim](https://github.com/frochet/routesim/tree/main) and currently requires the topology output from [MTG-Simulator
](https://github.com/sus0pid/MTG-Simulator), though I plan to generate these topologies here later so we don't need the MTG output. It
currently supports a simple user model and a simple hidden-service model.

The simulator reads a layout CSV file, treats each epoch column as one
topology snapshot, and samples user message paths through the active topology.
A route is marked compromised when every hop in the sampled path is malicious.

## Current Models

The simple model:

- simulates `--users` independent users
- samples one message at a time per user
- waits a uniformly random interval in `[5, 15)` minutes between messages
- optionally uses persistent vanguard and guard selection
- emits route records until `--days` virtual days have elapsed
- can print an aggregate compromise summary with `--summary`

The simple hidden-service model:

- simulates `--users` independent hidden services
- waits a uniformly random interval in `[5, 15]` minutes between client requests
- sends a uniformly random burst of `[1, 100]` messages for each request
- places all messages in a burst within a 60 second window
- uses the console request id column to identify the burst when `--to-console` is enabled

When persistent positions are enabled, routes use:

- hop 0: persistent vanguard sampled from layer 0
- hop 1: persistent guard sampled from layer 1
- hop 2: random weighted sample from layer 2

Use `--disable-vanguards` for guard-only routes:

- hop 0: random weighted sample from layer 0
- hop 1: persistent guard sampled from layer 1
- hop 2: random weighted sample from layer 2

Use `-d` to disable both persistent guards and vanguards.

The current codebase:

- `src/main.rs`: CLI, topology loading, runner setup
- `src/simulation.rs`: simulation runner, path sampling, compromise check, output
- `src/simplemodel.rs`: simple synchronous user behavior
- `src/simplehiddenservicemodel.rs`: simple hidden-service burst behavior
- `src/usermodel.rs`: minimal user-model traits, route event type, guard state
- `src/config.rs`: topology CSV parsing and weighted path sampling
- `src/mixnodes/`: mix node parsing/types

## Build And Test

```bash
cargo test
```

```bash
cargo run -- --help
```

## Example

```bash
cargo run -- \
  --topology-file testfiles/single_layout/1000_137_Random_BP_layout.csv \
  --model simple \
  --epoch 86401 \
  --users 2 \
  --days 1 \
  --to-console
```

Example output line:

```text
1970-01-01 00:44:31 2538 570,260,1007 false
```

That line contains the virtual timestamp, user id, sampled path mix ids, and
whether the sampled route was fully compromised.

For large runs, avoid route output and print only the aggregate summary:

simple model:
```bash
cargo run --release -- \
  --topology-file testfiles/layout_data/bow_tie/dynamic_hybrid_steady_0.03_layout.csv \
  --model simple \
  --epoch 3600 \
  --users 5000 \
  --days 30 \
  --summary
```

hidden service model:
```bash
cargo run --release -- \
  --topology-file testfiles/layout_data/bow_tie/dynamic_hybrid_steady_0.03_layout.csv \
  --model simple-hidden-service \
  --epoch 3600 \
  --users 5000 \
  --days 30 \
  --summary
```


## TODO
- [ ] generate topologies locally (this will probably best work for free-route because of bin-packing in the stratified mix from the paper)
- [ ] simulate free-route mix
- [ ] fix the selection logic for vanguards, current logic is the same as guard, Tor spec defines differently. Also vanguards are sampled from the first hop/layer and that layer doesn't have the guard maintenance logic, i.e., active guards, backup guards, down guards, etc. so it is less effective. 
- [ ] add more complex hidden services simulations
- [ ] add model for dead-drops/mailboxes, see approaches used in these paper:
  - [Vuvuzela](https://dl.acm.org/doi/pdf/10.1145/2815400.2815417)
  - [Alpenhorn](https://www.usenix.org/system/files/conference/osdi16/osdi16-lazar.pdf)
  - [Stadium](https://dl.acm.org/doi/pdf/10.1145/3132747.3132783)
  - [Karaoke](https://www.usenix.org/system/files/osdi18-lazar.pdf)
  - [Groove](https://www.usenix.org/system/files/osdi22-barman.pdf)
  - [PingPong](https://arxiv.org/pdf/2504.19566)