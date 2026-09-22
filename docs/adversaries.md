# Adversaries and persistent discovery

**Global Passive Adversary (GPA).** An adversary that is able to observe the network traffic globally.

**Mix Node Adversary (MNA).** An adversary that controls a fraction $`\beta`$ of the eligible Mix nodes. A path is considered fully compromised when every Mix node on the relevant path is controlled by the adversary.

**Adaptive adversary (AA).** An adversary that may compromise additional Mix nodes over time and may choose which nodes to target based on information learned from previously compromised nodes. This includes attacks against persistent paths or topologies in which the adversary attempts to progressively compromise nodes toward an endpoint.

## Compromise profiles
These profiles are taken from the [Tor's vanguard simulator](https://github.com/asn-d6/vanguard_simulator).

| Model | behavior |
| --- | --- |
| `Sybil` | Sybil a percentage $`\beta`$ of the mix nodes |
| `basic` | 50% chance of compromise within 15 days, otherwise never |
| `APT` | 75% within 15 days and 100% by 30 days |
| `FVEY` | 50% within 2 days, 75% within 7 days, otherwise never |
| `rubberhose1` | 50% between 2 and 14 days, otherwise never |
| `rubberhose2` | 50% between 7 and 21 days, otherwise never


# The adversary path walker

The path walker models an adversary that discovers a hidden service by following controlled mix nodes from the client-facing exits. It can move through the path by using its initially malicious node or tries to compromise nodes on the way. The adversary wins when it can follow a complete controlled route to the service.

[TODO: add walker logic, and more details ...]