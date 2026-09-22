# Simulator architecture

## Components
The `mixpathsim` simulator contains three main components:

- **the topology genrator**: takes a mixnet params and assumptions and generates a mixnetwork (set of mix nodes) for the required number of epochs (time-periods).
- **the user model**: describes the communication behaviour being simulated and defines the adversary, i.e., what it means for a user/service to be de-anonymized.
- **the path sampler**: implements the path-selection strategy being evaluated.


The rust crate contains:

| Source | Responsibility                                                                         |
| --- |----------------------------------------------------------------------------------------|
| [main.rs](../src/main.rs) | CLI choices, resolve profiles, create the network and models, and make summary/CSV.    |
| [mixnet.rs](../src/mixnet.rs) | Generate the shared static network with node identities and initial malicious control. |
| [params.rs](../src/params.rs) | Defaults.                                                                              |
| [path_sampler/](../src/path_sampler/mod.rs) | Path selection for simple and download models.                                         |
| [time_based_path_sampler/](../src/time_based_path_sampler/mod.rs) | Time-based selection, observations, and rotation events.                               |
| [adversary/](../src/adversary/mod.rs) | the adversary model which defines what it mean for user/service to be de-anonymized.   |
| [usermodel/](../src/usermodel/mod.rs) | Model-specific event streams modelling traffic behaviour.                              |
| [simulator.rs](../src/simulator.rs) | Run users in parallel, stop at first win, and merge results.                           |
| [summary.rs](../src/summary.rs) | Aggregate outcomes, print summaries, and export CSVs.                                  |
| [scripts/](../scripts/README.md) | Run the experiments and plot their CSVs.                                               |

## mixnet assumptions

The global mixnet is a set of identifiable nodes. Every node has a stable `MixId` and an initial malicious flag. A path is an ordered sequence of distinct nodes. In simple and download runs, a sampled path is compromised when every node on it is initially malicious. Hidden-service additionally model an adaptive adversary that discovers adjacent nodes through controlled nodes and can compromise more nodes over time.

The mix network is free-route: it has no global layer assignment. A sampler may create a private layered local topology for one user or service. Those local layers restrict that sampler's choices and do not affect the network.

## Simulation lifecycle

A `UserModel` produces `(time_seconds, adversary_won)` values through `fetch_next()`. The runner consumes this stream until its first win, deadline, or exhaustion. `None` ends that user's stream. The same tuple represents different things by model: a message, a download packet path, or a hidden-service event.

[TODO ...]

