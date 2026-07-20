# hs-mix-sim

This repository contains a Rust workspace for Monte Carlo route-compromise simulations in mix networks. The aim is to study and better understand the fully malicious path problem in decentralized mixnets. A sampled path is considered compromised when every node on that path is malicious. For more information on the problem, see [this research post](https://forum.research.logos.co/t/hidden-services-over-mix/706).

## Choosing a simulator

- [`routesim-v2`](crates/routesim-v2/README.md)  to simulate paths through existing [MTG-generated](https://github.com/sus0pid/MTG-Simulator), stratified topology layouts. This simulator is a simplified version of [routesim](https://github.com/frochet/routesim) with an extention of additional [Vanguards](https://spec.torproject.org/vanguards-spec/) similar to those use in Tor hidden services. 
- [`freeroutesim`](crates/freeroutesim/README.md) to simulate free-route mixnets and study time and message count to first compromise under random, guard, or vanguard routing. This is more relevant for [the mix protocol](https://lip.logos.co/anoncomms/raw/mix.html) we consider. 

This project builds on ideas and code from [routesim](https://github.com/frochet/routesim).
