# Adversaries 
In mixnets, threat models often consider the following adversaries:

**Global Passive Adversary (GPA).** An adversary that is able to observe the network traffic globally.

**Mix Node Adversary (MNA).** An adversary that controls a fraction $`\beta`$ of the eligible Mix nodes. A path is considered fully compromised when every Mix node on the relevant path is controlled by the adversary.

**Adaptive adversary (AA).** An adversary that may compromise additional Mix nodes over time and may choose which nodes to target based on information learned from previously compromised nodes. This includes attacks against persistent paths or topologies in which the adversary attempts to progressively compromise nodes toward an endpoint.

Our threat models mainly considers MNA and AA and the simulation provides stats that considers only these two adversaries.

## Compromise profiles
In the simulator, we consider these threat models or "compromise profiles". These profiles define the capabilities of the adversary. The following are taken from the [Tor's vanguard simulator](https://github.com/asn-d6/vanguard_simulator):

| Model         | behavior                                                 |
|---------------|----------------------------------------------------------|
| `Sybil`       | Sybil a percentage $`\beta`$ of the mix nodes            |
| `basic`       | 50% chance of compromise within 15 days, otherwise never |
| `APT`         | 75% within 15 days and 100% by 30 days                   |
| `FVEY`        | 50% within 2 days, 75% within 7 days, otherwise never    |
| `rubberhose1` | 50% between 2 and 14 days, otherwise never               |
| `rubberhose2` | 50% between 7 and 21 days, otherwise never               |

## The path walker (`PathWalker`)

The path walker models an adversary that discovers a hidden service by following controlled mix nodes from the client-facing exits. It can move through the path by using its initially malicious node or tries to compromise nodes on the way. The adversary wins when it can follow a complete controlled route to the service.

We implement the `PathWalker` as a struct that holds a state which persists between calls (all within the same model instnce being run) and manages compromise attempts.

### The walker can `peak`
The walker does not call `sample_path` repeatedly as simulation time move but rather it asks the sampler what it can observe using `peak()`. The peak function when called as `peak(&chain)` request the next nodes toward the service on routes matching the entire `chain`. If `chain` is empty, the sampler is expected to return the current observable exits (last nodes on the client side on any possible path that can be sampled). The path sampler can return any of the following:

```rust
pub enum Observation {
    Unavailable, // No current route supports this observation.
    Nodes(Vec<MixId>), // node (or exit) ids that can be observed from the given chain
    ServiceIdentified, // the given chain reaches the service
}
```

For example: consider a route `service → A → B → E → client`, the walker starts with `chain = []` and gets `[E]`, then it could advance and control that node and calls `peak(&[E])` and gets `[E, B]`, then `[E, B, A]`.

Note that the walker must first control the `chain` before asking for a `peak` using that chain. Therefore, `ServiceIdentified` represents a win through nodes controlled (through sybil or compromise) by the adversary.

## walker state

The walker state contains these values for the entire run:

- **Attempts:** vector of records, each with one record containing a `MixId`, and a state `Pending { completes_at }`, `NeverSucceeds`, or `Compromised { completed_at }`.
- **Budget and attempts by layer:** the maximum permitted attempts and how many have been used.
- **New completion events:** a buffer of events waiting to be handed to the model's scheduler.
- **stats:** completed-compromise count 
- **win flag**

### The walker loop logic

Recall that at each win check the model calls the adversary, the adversary then calls the walker to try to "walk" any of the paths and return either `true` (adversary win) or `false`. The logic is as follows:

1. If a previous check already won, return `true`.
2. Ask for exits with `peak(&[])`. 
3. Add each unique exit as a one-node chain to a queue containing chains.
4. Remove the next chain from the queue and check:
- if the nodes on that chain are not controlled (either through Sybil or compromised earlier), add them to a discovered set for its layer and stop exploring this branch. The walker tries to compromise the nodes on this discovered set. The compromise attempt success and failure depends on the adversary profile and compromise budget. 
- If the nodes are controlled, then the walker calls `peak` with the whole chain. An `Unavailable` result ends this walk through this chain. A `ServiceIdentified` result records a win and returns immediately. Otherwise, append each returned node in `Nodes(Vec<MixId>)` to the chain and add it to the chain queue.
5. Repeat until the queue is empty. If no chain won, `false`.

notes:
- `seen` set: tracks chains because the same node can occur in different route contexts. 
- `discovered` set: tracks discovered node ids to avoid duplicates because several chains could lead to a single node.
- A node is controlled when its initial `is_malicious` flag is true **or** its attempt record is `Compromised`. Initial Sybils don't need to be compromised.
- a call to the walker always starts the walk from the exists but the walker state provides context on which additional nodes the adversary controls through compromise. Sybil is easy to check using the mix node id.

In pseudocode:

```text
check_win(now, sampler):
    if already_won: return true
    exits = sampler.peak([])
    if exits is Unavailable: return false
    if exits is ServiceIdentified: record win; return true

    queue = unique one-node chains for exits
    seen = those chains
    discovered = empty map from layer to set of node IDs

    while queue is not empty:
        chain = queue.pop_front()
        node = sampler.node(chain.last())
        if node is missing: continue

        if not is_controlled(node):
            layer = sampler.hops() + 1 - length(chain)
            discovered[layer].insert(node.id)
            continue

        observation = sampler.peak(chain)
        if observation is ServiceIdentified:
            record win
            return true
        if observation is Nodes:
            for next_node in observation:
                extended = chain + [next_node]
                if extended is new in seen:
                    queue.push_back(extended)

    attempt_discovered(discovered, now, compromise_profile)
    return false
```

## Compromise attempts

For each discovered layer, the walker first removes nodes with previous attempt records. Based on the configured budget, the walker will compute the number of attempts remaining. It will then choose up to that many candidates uniformly.

The current default is `CompromiseBudget::Limited(1)`: at most one attempt per layer over the entire simulation. `Unlimited` removes that limit. Failed attempts still count in the budget, and initial Sybils are free. The `Sybil-only` adversary uses a zero attempt budget.

## How path walker generalizes across path-selection strategies

Each time-based sampler uses its state and expose the observation interface (`peak`):

| Strategy                                  | What the walker explores                                                  |
|-------------------------------------------|---------------------------------------------------------------------------|
| Fixed paths                               | Chains matching currently stored complete paths.                          |
| Fixed topology                            | Chains allowed by the current layered local-topology and its connections. |
| Fixed paths over a fixed topology (FPOFT) | Chains matching the current active paths.                                 |
By simply enabling the walker to peak, we eliminate the need sampling single paths since we assume the adversary could have unlimited number of sessions with the service. A new strategy can reuse the adversary logic by implementing this simple `peak` API: return current exits, validate the given chain, reveal only adjacent nodes, and report service identification only when a complete chain (basically a path at this point) leads to the service. The walker can then use this API to walk the path. Note that this works only for time-based path samplers since they apply rotation to active paths or nodes on the local topology. If random selection is used for any of the hops on the path, we can basically ignore that hop and consider it compromised since again we assume the adversary can have unlimited number of sessions with the service causing it to sample unlimited number of paths meaning it is guaranteed that an adversary mix node would be used for that hop.
