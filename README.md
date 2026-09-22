# mixpathsim

This repository contains a Rust simulator for route-compromise simulations in free-route mix networks. The aim is to study and better understand the fully malicious path problem in decentralized mixnets. A sampled path is considered compromised when every node on that path is malicious. For more information on the problem, see the research posts: 
- [Hidden Services over Mix](https://forum.research.logos.co/t/hidden-services-over-mix/706).
- [Mix Path Selection](https://forum.research.logos.co/t/mix-path-selection/721)
- [Hidden Service (Time-Based) Path Selection](https://forum.research.logos.co/t/hidden-service-time-based-path-selection/730)

The models we simulate here are:

- **Simple:** users send messages over time and measures cumulative compromise probability by message count and time.
- **Hidden service:** discrete-event simulation that follows path rotations and compromise events to measure service-identification probability over time.
- **Download:** simulates anonymous downloads and measures cumulative compromise probability (S-DLM) by download sizes and time.

## How to run

You need Rust/Cargo. For plots, also install Python 3 and the plotting dependency:

```bash
python3 -m pip install -r scripts/requirements.txt
```

Edit [scripts/params.sh](scripts/params.sh) to choose your simulation settings. Set `PLOT=true` to generate plots or `PLOT=false` for CSVs and logs only; Python is unnecessary when plotting is disabled.

From the repository root, run the script for each model:

```bash
bash scripts/run_simple.sh
bash scripts/run_hidden_service.sh
bash scripts/run_download.sh
```

Run whichever model you need. See the [parameter reference](scripts/README.md) for all settings, sampler choices, presets, and adversaries.

## Docs

see [results doc](./docs) for documentation and results on all experiments. 
