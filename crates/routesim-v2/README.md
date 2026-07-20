# routesim-v2

`routesim-v2` is a simulator for stratified mix. It reads one or more layered topology snapshots from an [MTG Simulator](https://github.com/sus0pid/MTG-Simulator) CSV file and simulates independent users sending messages.

Every path contains three hops (mtg only generates 3 layer), one from each layer. Nodes are sampled in proportion to their bandwidth, and a message path is considered compromised only when all three selected nodes are malicious.

## Topology input files

The first three CSV columns describe each mix:

```text
mix_id,bandwidth,malicious
```

- `mix_id` is an integer identifier.
- `bandwidth` is the node's sampling weight.
- `malicious` is `true` or `false`.

The remaining columns assign that mix to a layer in each topology epoch. Layer values `0`, `1`, and `2` place a node in the corresponding route layer. `-1`, mean that the node is not active/selected in that epoch.

Example file containing one epoch:

```csv
mix_id,bandwidth,malicious,layer
0,7.2,false,-1
1,13.0,false,2
2,9.5,true,0
```

For multiple epochs:

```csv
mix_id,bandwidth,malicious,epoch_0,epoch_1,epoch_2
0,7.2,false,0,-1,1
1,13.0,false,1,1,2
2,9.5,true,2,0,-1
```

Each epoch is valid for `--epoch` seconds. See [testfiles](./testfiles) for the test files used in the paper. 

## Guards and vanguards

Guards and vanguards are enabled by default:

- hop 0 uses a persistent vanguard from layer 0
- hop 1 uses a persistent guard from layer 1
- hop 2 is sampled by bandwidth from layer 2

Each user maintains its own guard/vanguard sets. When the topology changes, it keeps the first candidate still present in the appropriate layer. If none are present, it extends the set from the current epoch and selects a new node.

Use `--disable-vanguards` for only the guard. Use `-d` to disable both guard and vanguards and sample all three layers by bandwidth.

## Models

### `simple`

The default model sends one message after each uniformly sampled interval in `[300, 900)` seconds.

### `simple-hidden-service`

The simple hidden-service model:

- receives a request after each uniformly sampled interval in `[300, 900]` seconds.
- sends a burst of between 1 and 100 messages for the request.
- places those messages at random offsets in the following 60 seconds.

Select it with `--model simple-hidden-service`.

## Running the simulator

Run these examples from the workspace root.

Print an aggregate summary for the included single epoch topology:

```bash
cargo run -p routesim-v2 --release -- \
  --topology-file crates/routesim-v2/testfiles/single_layout/1000_137_Random_BP_layout.csv \
  --model simple \
  --epoch 86401 \
  --users 5000 \
  --days 1 \
  --summary
```

Print every hidden-service route while keeping guards but disabling vanguards:

```bash
cargo run -p routesim-v2 --release -- \
  --topology-file <layout.csv> \
  --model simple-hidden-service \
  --epoch 3600 \
  --users 5000 \
  --days 30 \
  --disable-vanguards \
  --to-console
```

View every option with:

```bash
cargo run -p routesim-v2 -- --help
```

## Plotting first-compromise results

The plotting script requires Python 3 and Matplotlib.

From the workspace root:

```bash
python3 crates/routesim-v2/scripts/plot_v2_console.py \
  --format simple \
  --users 5000 \
  --in-file <routes.txt> \
  --out-prefix <output-prefix>
```

The helper scripts run the simulation and plotting step together:

```bash
crates/routesim-v2/scripts/run_simple_plot.sh <layout.csv> <output-directory>
crates/routesim-v2/scripts/run_hidden_service_plot.sh <layout.csv> <output-directory>
```

## limitations

- only stratified mix.
- required the MTG simulator output.
- Paths have a fixed length of three hops.
- sampling is always based on bandwidth (as in the paper). 
- Guard and vanguard maintenance is a simplified persistence model, not a complete Tor implementation.

Most of the code is from [routesim](https://github.com/frochet/routesim). See [freeroutesim](../freeroutesim) which doesn't have these limitation. 
