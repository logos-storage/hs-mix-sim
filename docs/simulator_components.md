# Simulator architecture

## Components
The `mixpathsim` simulator contains three main components:

- **the mixnet generator**: creates one static set of mix nodes and assigns their initial malicious flags, shared by all users in a simulation.
- **the user model**: describes the communication behaviour being simulated. I.e., it defines the number of paths/packets required and the timeline for when they are sampled/selected. 
- **the path sampler**: implements the path-selection strategy being evaluated.
- **the adversary**: defines the adversary model, i.e., the logic/steps that the adversary would follow to try to de-anonymized the user/service.

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

## Simulation experiments

An experiment evaluates a number of users or services with one shared mixnet. Each user has its own simulation state.

### Inputs

The simulation require some input (taken from the CLI):
- umber of users or services we want to simulate. More would result in better estimate of compromise probability.
- Simulation duration in days
- Mixnet size and malicious fraction
- Model, adversary, and path selector that we want to simulate. These could have configuration options as well which we need to pass. 
- Simulation output settings, i.e., if we want plots, CSV and summary.

### Simulation steps

1. **Read and validate the configuration.** Read the arguments, apply defaults and profiles, and check that the selected options are compatible.
2. **Generate the mixnet.** Creates a single static network of `N` nodes with distinct, stable identities. For a configured malicious fraction `beta`, it selects `ceil(N × beta)` distinct nodes uniformly without replacement and marks them as initially malicious. This network is shared throughout the experiment.
3. **Initialize each user's run.** Construct separate model, sampler, and adversary state for every user, with access to the shared mixnet.
4. **Run users in parallel.** Each worker processes one user's sequence in order. The worker repeatedly calls the model with `fetch_next()`, which returns either `(time_seconds, adversary_won)` or `None`.
5. **Record outcomes and stop each run.** If a returned timestamp `time_seconds` exceeds the deadline or `adversary_won` is `true`, stop the simulation. Otherwise, record the result. 
6. **Aggregate the results.** get the outcome results from all user runs and generate the statistics.
7. **Write the outputs.** Print the experiment summary and, when requested, export the results to CSV. Plotting is done using a separate script that reads the CSV.


