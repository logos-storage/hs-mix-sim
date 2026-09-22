# Research context and scope

In this research, we try to understand the malicious path problem and how much anonymity we can expect when a long-term connection/session is maintained over the Mix protocol. Examples of such sessions are anonymous downloads and hidden services.

We basically want to understand two things:
1. What anonymity guarantees can we expect on long-term connections/sessions? and how bad is this in practical settings? We can theoretical compute this with mathematical formulas and bounds and we can simulate this to get some numbers as well.
2. Can we improve (i.e., decrease) the probability of de-anonymization by changing the path selection strategy? We can design path selection strategy for our use-case and mixnet setting by building on prior work which include: the Tor [Guard](https://spec.torproject.org/guard-spec/index.html) and [Vanguard](https://spec.torproject.org/vanguards-spec/) protocols, the [Bow-tie](https://dl.acm.org/doi/pdf/10.1145/3564625.3567996) strategy, and the other recent strategies proposed in the [NDSS paper](https://www.ndss-symposium.org/wp-content/uploads/2026-f2384-paper.pdf).

## Motivation
There are two main motivations, both require sampling/selecting large number of paths:

1. anonymous download:
An anonymous download protocol can be built using the Mix protocol as it is now, with an additional transport layer, i.e. a protocol for handling sending large messages, integrity, reliability, and SURBs management. However, anonymous download over mix would require maintaining a session. We can define a session as a single or multiple transport layer sessions where a client requests and receives multiple chunks/files that are related, i.e., the chunks are all related to the same file or multiple files but all belong to the same content category/type.

In a session, the two communicating parties will exchange multiple packets and for each one, they will select a path. If we assume a pecentage of the network is controlled by malicious mix nodes (e.g., 10%), then the more packets needed for a session, the more chance that a path containing all malicious nodes is possible. Therefore, if we expect traffic in the mix network to consist of sessions, then we need to consider the malicious mix adversary in our threat model and design path selection strategies to lower the probability of de-anonymization. 

2. hidden services:

A hidden service protocol is useful for many applications in which services need to serve content and stay anonymous. One of the main motivation is supporting logos-storage anonymous file-sharing. A hidden service protocol over Mix such as the one specified in this document would allow providers to be reachable and serve contents while at the same time stay anonymous.

However, a public service can face repeated malicious interactions, making accumulated exposure over time more relevant than the risk of one isolated packet. This requires a path selection strategies that considers time rather than sessions, and we refer to these strategies as "time-based strategies".

## Notation

| Symbol                         | Meaning                                                                                                 |
|--------------------------------|---------------------------------------------------------------------------------------------------------|
| $\beta$                        | malicious-node fraction.                                                                                |
| $L$                            | Number of mix hops on a complete path.                                                                  |
| $L_f$                          | number of hops controlled by the path selector. These could be fixed to one node or has a candadit list. |
| $K$                            | Candidates per fixed pool, when all pools have the same size.                                           |
| $K_i$                          | Candidate count in local topology layer $i$.                                                            |
| $d$                            | Degree or distinct outgoing connections per node to the next layer.                                     |
| $M$                            | Number of stored active complete paths.                                                                 |
| $N$                            | Number of packet paths in a session.                                                                    |
| $T$                            | hidden service lifetime.                                                   |
| $b$                            | Maximum compromise attempts per layer over one run.                                                     |

### Topology notations
The paths with fixed hops and sets would look like this:

```
Sender -> [L1] -> [L2] -> [L3] -> ... -> R -> receiver
```

where the size of these set is: K1, K2, and K3. R is a random mix node. The choice of the number of hops and the size of each set would result in different probabilities of de-anonymization. We will refer to this as the topology and use the notation x-x-...-x, where x is the size of the set. e.g., 5-5-5 would mean 3 hops, with each having 5 possible nodes to select from.

In some cases we will use this notation `R5` in a time-based topology which means five candidate slots whose occupants rotate independently. 

