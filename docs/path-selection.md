# Path selectors

Path selectors decide which mix nodes form a path and how those choices persist across requests. We separate session from time-based path selectors since the latter requires additional functionalities.

## Session path selectors

Session path selectors provide simple interface with `sample_path()` which returns a path. The path selector can use any strategy and can maintain a state. We have implemented 4 path selector strategies:

| Selector       | Strategy                                                                                                                                   |
|----------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `random`       | Sample a fresh uniform path.                                                                                                               |
| `k-hf`         | Keep one node at each selected fixed position, random selection for the other positions.                                     |
| `k-w`          | Keep a candidate pool at each selected fixed position; choose one node from each pool per request and randomly select nodes for the other positions. |
| `alpha-sticky` | Introduce a first path. Later, with probability alpha choose uniformly from the previously used paths, otherwise add a new path.           |

### `random`
Every hop is sampled uniformly from active nodes without repeating a node within the path. A fresh path is sampled for every message. This is the currently used path selection strategy used in the mix protocol and will serve as base-line for other strategies. We can use the same $\texttt{S-DLM}$ formula for this

$$
\texttt{S-DLM}_{\mathrm{UR}}(N)
\approx
1-\left(1-\beta^L\right)^N.
$$

Note we are ignoring the small without-replacement difference here.

### `k-hf`
The idea is that K-HF splits the $L$ hops path into two parts:
- $h_f$ fixed positions. At each position we maintain a single node that stays unchanged for the whole session.
- $L-h_f$ positions with nodes that are randomly resampled for every packet.

Given these conditions, for a malicious path to be chosen, two things must happen:
1. All fixed nodes are malicious. Since each fixed node has probablity $\beta$ of being malicious, then this happens with probability $\beta^{h_f}$. If a single fixed position contains all honest nodes, then the session will never have a malicious path.

2. The remaining $L-h_f$ nodes that are not fixed are all also malicious. The probability that all $L-h_f$ randomly chosen nodes are malicious is $\beta^{L-h_f}$, therefore, the probability that at least one of the paths in the non-fixed positions in a session ( with $N$ paths selected) contain all malicious nodes is $1-\left(1-\beta^{L-h_f}\right)^N$ which is basically the same as $\texttt{S-DLM}$ above since we are randomly selecting nodes for these positions.


Multiplying the two required events gives us the formula for  this path selection strategy:
$$
\texttt{S-DLM}_{\mathrm{K\text{-}HF}}(N)
\approx
\beta^{h_f} \cdot
\left(1-\left(1-\beta^{L-h_f}\right)^N\right)
$$

we can see here that for large $N$:

$$
\left(1-\beta^{L-h_f}\right)^N \to 0
$$

so:

$$
1-\left(1-\beta^{L-h_f}\right)^N \to 1
$$

meaning that what we compute is eventually just $\beta^{h_f}$ which is the probability that our initial pick for these fixed hops is fully malicious. For example, fixing a single hop in the path and assuming large number of packets/path sampled ($N$), then the probability or de-anonymization during that session comes down to whether or not the fixed hop is malicious, since in mix as long as a single hop in the path honest, anonymity is preserved. Therefore, if we assume malicious control is $\beta = 0.1$, then there is 10% chance of deanonymization with the single fixed hop. we can lower that with fixing more hops, e.g., two fixed hops and $0.1^2 = 0.01$ then 1% chance of deanonymization.

### `k-w`
In this strategy, we create persistent sets containing $K$ uniformly selected nodes from the online/live mix nodes $W$ for each hop position. i.e., we will have $L$ sets, each of size $K$, so $S = (s_1,s_2,\ldots,s_L,)$ where $|s_j|=K$. Note that here $W=m$ because we are considering a free-route mix and so we sample from all online mix nodes and note from layers as in stratified mix.

For a fully malicious path to be possible, we need the following two conditions to happen:

1. every set in $S$ must contain at least one malicious node. If a single set doesn't contain a malicious node, then malicious paths will never happen.
2. One of the session's $N$ packets actually selects malicious nodes from all pools simultaneously.

The first condition requires modeling the malicious fraction inside each $K$-node pool using the hypergeometric distribution. This gives us the probability of selecting a malicious node from the pool in $s_j$:

$$
\beta_j
\sim
\texttt{Hypergeometric}(W,\beta W,K)
$$

where $\beta_j$ is the malicious fraction in set $s_j$, $W$ is the number of nodes from which the set is sampled, $\beta W$ is the number of malicious nodes, and $K$ is the set size. Again in our setting, $W=m$.

We use the hypergeometric distribution because each fixed set contains $K$ distinct nodes sampled *without replacement* from the mix pool with a known malicious fraction $\beta$. The hypergeometric distribution therefore models how many malicious nodes end up in each set.

Now, to get the probability that one packet selects a fully malicious path is basically:

$$
q = \prod_{j=1}^{L}\beta_j
$$

For a session of $N$ packets, the probability that at least one packet is compromised (fully malicious) is just plugging $q$ into $\texttt{P-DLM}$:

$$
1-\left(1-q\right)^N =
1- \left( 1- \prod_{j=1}^{L}\beta_j \right)^N
$$

Because the values $\beta_j$ depend on the randomly generated sets, we need to compute the expected value over their possible combinations. Therefore, the resulting formula for the probability as used in the paper is:

$$
1-\mathbb{E} \left[
\left(1-\prod_{j=1}^{L}\beta_j\right)^N
\right]
$$

However, as suggested by the paper, we can use a simpler upper bound approximation:

There are at most $K^L$ distinct paths that can be formed from the fixed sets. A session with $N$ packets can therefore use at most $\min(N,K^L)$ distinct paths.

Approximating each distinct path as having compromise probability $\beta^L$ (similar to a randomly selected path, but in reality it should be less because paths selected from fixed sets are not truly independent), we get:

$$
\texttt{S-DLM}_{\mathrm{K/W}}(N)
\lesssim
1-
\left(1-\beta^L\right)^{\min(N,K^L)}
$$

Observe that this is basically the same as the formula for $\texttt{S_DLM}$ and only improves it when $K^L < N$. We will use this approximation formula for our evaluation of this path selection strategy, but simulation numbers are expected to be less than this upper bound.

### `alpha-sticky`
Alpha-SS strategy maintains a set $S_\alpha$ which contains the paths used to send packets through mix.

The set $S_\alpha$ starts empty and so you sample the first path uniformly and add it. For every later packet it:
- Sample a path from $S_\alpha$ with probability $\alpha$ or
- samples a new, previously unused path from the online mix nodes with probability $1-\alpha$. This path is used and added to $S_\alpha$.

Let $q=\beta^L$ be the probability that a uniformly selected path is fully malicious, then the paper suggests this formula:

$$
\texttt{S-DLM}_{\alpha\text{-SS}}(N)
\approx
1-(1-q)
\prod_{i=2}^{N}
\left[
\alpha+
(1-\alpha)
\left(
1-
\frac{m^L q}
{m^L-1-(1-\alpha)(i-2)}
\right)
\right]
$$

This looks quite complex, but let's try to unpack it:

- when you have no path in $S_\alpha$ and you pick one randomly there is $q=\beta^L$ chance of that path being malicious. So it will be safe with probability $1-q$.
- we re-use existing paths with probability $\alpha$
- new path are selected with probability $1- \alpha$, and the there is let's called it $q'$ chance that these paths are malicious. The paper computes this $q'$ as:
  $$
  q' = \frac{m^L q}{m^L-1-(1-\alpha)(i-2)}
  $$
  what this fraction basically says is that we are dividing the number of malicious paths ($m^L q$) by the total number of paths after removing the ones we selected ($m^L-1-(1-\alpha)(i-2)$). What we are removing is: $1$ for the first path, $(1-\alpha)(i-2)$ the appoximate number of path we selected previously which depends on $\alpha$.

$$
m^L - \underbrace{1}_{\text{first selected path}} -
\underbrace{(1-\alpha)(i-2)}_{\substack{\text{expected number of additional}\\\text{new paths selected earlier}}}
$$

- Now if we put all the previous ones together, we get the paper's suggested formula.


However, if we simplify and assume each path is sampled independently, i.e., $q' = q$ (meaning we are ignoring the fact that the path pool we are selecting from gets smaller and smaller as we sample more paths), then we have:

$$
\alpha+(1-\alpha)(1-q)
= 1-(1-\alpha)q
$$

This simplifies the formula to:
$$
\texttt{S-DLM}_{\alpha\text{-SS}}(N)
\approx
1-(1-q)\left(1-(1-\alpha)q\right)^{N-1}
$$

### Session profiles
To simplify the anonymity requirement, the path selection spec uses three anonymity profiles. The CLI provides these three K/W profiles through `--download-profile`:

- `LITE` favors path diversity and availability by fixing fewer hops and having large candidate sets. It is the least likely to interrupt a session because of unavailable candidates.
- `STANDARD` is the default profile and balances path diversity, availability, and exposure to malicious candidates.
- `STRICT` prioritizes anonymity and limiting exposure to new nodes/candidates at the cost of lower path diversity and a greater chance that the session might end when candidates are unusable/offline.

Each profile determines:

- the path length $`L`$
- the number of fixed hop positions $`L_f`$
- the candidate-set size $`K`$

| Profile              | $`L`$ | $`L_f`$ | $`K`$ |
|----------------------|------:|--------:|------:|
| `LITE` (`5-R-R`)     |     3 |       1 |     5 |
| `STANDARD` (`5-5-R`) |     3 |       2 |     5 |
| `STRICT` (`3-3-3-R`) |     4 |       3 |     3 |

All three profiles use the same policy as K/W selector.

Assuming $`\beta_f=0.1`$ and $`N = \infty`$, the resulting probability of de-anonymization for each profile are:


| Profile | $`\texttt{S-DLM}`$ |
|---|---:|
| `LITE` (`5-R-R`) | 41% |
| `STANDARD` (`5-5-R`) | 17% |
| `STRICT` (`3-3-3-R`) | 2% |

## Time-based path selectors
Time-based selectors extend the session selector interface with scheduled events and observations of current routes. They also use lifetimes for either paths or nodes used on paths. Lifetime can be either `Never`, which produces no expiry event, or:

$$
\tau=\max(X_1,X_2),\qquad X_1,X_2\text{ uniform sampling on }[a,b]
$$

### Fixed-paths
5 active paths at any time, each path has $\max(X,X)$, $X\sim U(1,48)$ hours rotation/lifetime. A request chooses uniformly among the five stored paths. On expiry, only that path is replaced with a newly sampled path and lifetime. So we have:

```
active paths:
    +-- Path A: A1 -> A2 -> A3 -> A4
    +-- Path B: B1 -> B2 -> B3 -> B4
    +-- Path C: C1 -> C2 -> C3 -> C4
    +-- Path D: D1 -> D2 -> D3 -> D4
    +-- Path E: E1 -> E2 -> E3 -> E4
```

The expected de-anonymization probabilities after running the simulation:

| Model | Success by day 30 |
|---|---:|
| Sybil only | 1.111% |
| Basic | 2.712% |
| APT | 5.006% |
| FVEY | 29.463% |
| Rubberhose 1 | 1.109% |
| Rubberhose 2 | 1.109% |

There is still room to increase the number of possible paths if needed for availability or path diversity:

| Adversary    | 1 path | 5 paths | 8 paths | 9 paths | 10 paths |
|--------------|-------:|--------:|--------:|--------:|---------:|
| Sybil only   | 0.223% |  1.111% |  1.772% |  1.991% |   2.210% |
| Basic        | 0.548% |  2.712% |  4.304% |  4.829% |   5.350% |
| APT          | 1.022% |  5.006% |  7.888% |  8.830% |   9.761% |
| FVEY         |  6.70% |  29.46% |  42.68% |  46.61% |   50.21% |
| Rubberhose 1 | 0.223% |  1.109% |  1.768% |  1.987% |   2.206% |
| Rubberhose 2 | 0.223% |  1.109% |  1.768% |  1.987% |   2.206% |

We can try to write a formula to compute an estimate for Sybil-only.
Let:
- $m$: number of complete paths kept active at any time
- $L$: number of fixed hops in each path
- $\beta$: probability that a sampled node is malicious
- $T$: hidden-service lifetime
- $\tau$: lifetime of one complete path
- $N(T)$: total number of path generations sampled by time $T$.

Over time $T$, each path is expected to rotate $1+\frac{T}{\tau}$ times. With $m$ active paths, the number of paths sampled is:

$$
N(T) = m \left(1+\left\lfloor\frac{T}{\tau}\right\rfloor\right)
$$

A complete $L$-hop path is malicious only when all $L$ nodes are malicious:

$$
q=\beta^L
$$

The probability that one path sampled is not malicious is:

$$
1-\beta^L
$$

If the service samples $n$ independent paths, the probability that none are malicious is:

$$
(1-\beta^L)^n
$$

Taking the complement gives the probability of at least one malicious path:

$$
P_{\mathrm{Sybil}} = 1-\left(1-\beta^L\right)^n
$$

Using the expected number of paths from $N(T)$ gives the approximation:

$$
P_{\mathrm{Sybil}}(T)
\approx
1- \left(1-\beta^L\right)^{
m \left(1+\frac{T}{\tau}\right)
}
$$

If we try to plug in the params from before:
- $m=5$ active paths
- $L=4$ fixed hops
- $\beta=0.10$
- $T=30$ days $=720$ hours
- $\tau \approx 32.33$ hours (this is the expected value when using $\max(X,X)$, $X\sim U(1,48)$ hours rotation)

then we get:

$$
P_{\mathrm{Sybil}}(30\text{ days}) \approx 1.16\%
$$

The simulation produced 1.111%, which is close to this approximation.

### Fixed topology
`FixedTopology` constructs a local layered topology for one service by sampling distinct nodes for all layers. Layers run from the service toward the recipient: layer 1 is nearest the service, and the last layer contains the exits. The global mixnet itself remains free-route.

Adjacent layers use one of two connections:

- **Mesh:** every node connects to every node in the next layer.
- **Degree $d$:** every non-final node has exactly $d$ distinct outgoing neighbors in the next layer. Connection sampling first covers destinations with no incoming link, then fills remaining links randomly.

A local topology with `L` layers, `m` nodes per layer, and degree `d` results in different anonymity and availability guarantees. An example local topology:

```
               Fixed 5-5-5 topology, degree d=2

      L1                    L2                    L3

     [A1] ───────────────► [B1] ───────────────► [C1]
       └─────────────────► [B2] ───────────────► [C2]

     [A2] ───────────────► [B2] ───────────────► [C2]
       └─────────────────► [B3] ───────────────► [C3]

     [A3] ───────────────► [B3] ───────────────► [C3]
       └─────────────────► [B4] ───────────────► [C4]

     [A4] ───────────────► [B4] ───────────────► [C4]
       └─────────────────► [B5] ───────────────► [C5]

     [A5] ───────────────► [B5] ───────────────► [C5]
       └─────────────────► [B1] ───────────────► [C1]
```

For each path request, the selector chooses a first-layer node uniformly and then one outgoing link uniformly at each layer. All routes through the local topology are possible and can be selected.

Increasing the degree would increase the number of possible paths and limit a single failed node on the path to render that path unusable. If we consider these topologies:
- `5-5-5`: three fixed layers/hops each containing 5 nodes
- `5-5-5-5`: four fixed layers/hops each containing 5 nodes

Then increasing the degree would give us:

| Topology | Degree 1| Degree 2 | Degree 3 | Degree 4 | Degree 5 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `5-5-5` | 5 | 20 | 45 | 80 | 125 |
| `5-5-5-5` | 5 | 40 | 135 | 320 | 625 |

or we can just compute the number of paths:

$$
R = m \cdot d^{L-1}
$$

where:
- $m$ is the number of nodes in each layer
- $L$ number of layers
- $d$ degree

### Active paths over a fixed topology (`FPOFT`)

`FPOFTSampler` uses the same layered local topology, but restricts sampling to $M$ active complete routes. It initially selects these routes uniformly without replacement. Each request chooses one active route uniformly, and each active route has its own lifetime.

On path expiry, the sampler chooses uniformly among routes not used by the other active slots. It may select the expired route again. This preserves $M$ distinct active routes while allowing reuse over time.

Note that routes refer to stable topology slots rather than node ids. If a topology node rotates, all active routes using its slot immediately use the replacement. Their path timers remain unchanged. Conversely, rotating a path does not reset any node timer.


### Time-based Profiles
The current `fpoft` profiles are defined in [profiles.rs](../src/time_based_path_sampler/fixed_topology/profiles.rs):

| Profile               | $`L_f`$ | $`K`$ | $`d`$ | $`M`$ | $`R`$ | num of fixed nodes | Path rotation ($`\tau_{min}`$,$`\tau_{max}`$) |
|-----------------------|--------:|------:|------:|------:|------:|-------------------:|-----------------------------------------------|
| `LITE` (`5_5_R`)      |       3 |     5 |  mesh |     5 |     0 |                 10 | $`(1,48)`$ hours                              |
| `STANDARD` (`5_5_R5`) |       3 |     5 |     3 |     5 |     5 |                 10 | $`(1,48)`$ hours                              |
| `STRICT` (`5_5_5_R5`) |       4 |     5 |     3 |     5 |     5 |                 15 | $`(1,48)`$ hours                              |

All three profiles give each active path an independent max-of-two 1–48-hour lifetime. $`R`$ refers to a hop with random mix node sampled from the mixnet, whereas, $`R5`$ refers to a fixed set of randomly selected mix nodes, each with an independent lifetime sampled from the same $`\tau_{min}`$ and $`\tau_{max}`$ range.

`STANDARD` and `STRICT` place an additional fixed layer of rotating nodes $`R5`$ at the client-facing side. The purpose of $`R5`$ layer is to incentivize the adversary to sybil that layer and give the service more time to operate. This is because exit nodes are sampled from the general mixnet pool and rotate slowly, therefore, making it more attractive for the adversary to sybil attack instead of the more expensive compromise attack. Additionally, rotation limits how long a sybiled exit remains useful in its position, potentially reducing the usefulness of compromises that finish after it rotates.

The $`R5`$ layer follows these rules:
- Initialize five distinct nodes, excluding all nodes in the other layers.
- Assign each node an independent lifetime equal to the maximum of two uniform draws between 1-48 hours.
- Connect the preceding layer to $`R5`$ using the topology's configured degree.
- When a node expires, replace it with a uniformly selected eligible node outside the current topology. The replacement inherits the connections and active paths that used the expired node.
- Keep node and path timers independent: replacing a node does not reset path timers, and rotating a path does not reset node timers.

Simulations with malicious control $`\beta=0.10`$, a 30-day hidden service lifetime, five active paths, and 5,000 trials show the following expected $`\texttt{T-DLM}`$ and median time to service identification. `NR` means that the 50% threshold was not reached within the 30-day observation period.

| Profile | Sybil only | Basic | APT | FVEY | Rubberhose1 | Rubberhose2 |
|---|---:|---:|---:|---:|---:|---:|
| `LITE` (`5_5_R`) | 16.60% / NR | 48.34% / NR | 84.36% / 14.39 d | 72.54% / 5.64 d | 48.96% / NR | 43.42% / NR |
| `STANDARD` (`5_5_R5`) | 10.10% / NR | 27.62% / NR | 44.30% / NR | 55.52% / 24.04 d | 26.82% / NR | 20.56% / NR |
| `STRICT` (`5_5_5_R5`) | 1.52% / NR | 6.84% / NR | 12.44% / NR | 24.46% / NR | 6.10% / NR | 3.90% / NR |


### Vanguard profiles
We have added additional profiles to experiment with the vanguard approach, and compare it with other strategies. `vanguard1` and `vanguard2` use `FixedTopologySampler` with mesh connections. See the [Tor Vanguards specification](https://spec.torproject.org/vanguards-spec/) and [mesh-vanguards proposal](https://spec.torproject.org/proposals/292-mesh-vanguards.html).

| Profile | Layer sizes, service to recipient | Connections | Layer 1 node lifetime | Layer 2 node lifetime | Layer 3 node lifetime | Complete routes |
| --- | --- | --- | --- | --- | --- | ---: |
| `vanguard1` | `2-4-6` | Mesh | 90–120 days | 30–60 days | 1–48 hours | 48 |
| `vanguard2` | `2-4-8` | Mesh | 90–120 days | 30–60 days | 1–48 hours | 64 |

Every lifetime in this table uses the maximum of two independent uniform draws. At initialization, every node receives a fresh full lifetime. Each request can select any complete route through the mesh.

Experiments with 5,000 trials, 1,000 mix nodes with 10% initially malicious, and a 30-day observation period. Each compromising adversary has a budget of one attempt per layer for the entire run. These settings match the experiment settings for the LITE/STANDARD/STRICT results above. The results are:

| Profile | Sybil only | Basic | APT | FVEY | Rubberhose1 | Rubberhose2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `vanguard1` (`2-4-6`, mesh) | 6.24% / NR | 38.52% / NR | 79.50% / 18.30 d | 65.68% / 7.44 d | 40.98% / NR | 32.30% / NR |
| `vanguard2` (`2-4-8`, mesh) | 6.80% / NR | 38.72% / NR | 81.18% / 17.16 d | 66.78% / 6.35 d | 39.48% / NR | 33.74% / NR |

