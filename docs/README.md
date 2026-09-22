# Free-route mixnet path selection

Draft documents on the research done on mix path selection especially in the context of anonymous download and hidden services. Additionally, will include docs on the simulator `mixpathsim`.

### Summary
When considering a percentage of the mixnet being controlled by an adversary, then the probability of selecting a fully malicious path can become a large over time, especially when we try to maintain a session that requires sending many related packets. Long-lived services face an additional risk: an *adaptive* adversary can discover persistent intermediate nodes and attempt to compromise them over time. Path selection strategies therefore try to minimize exposure to malicious paths and the adaptive adversary. 

`mixpathsim` is a simulator that combines both Monte Carlo simulations and discrete-event simulations. The Monte Carlo simulations are useful for download/session path selection strategies and discrete-event simulations are more appropriate for hidden services models which operate over time. 

### Docs

See the following documents:
- [Research context and scope](research-context.md)
- [Simulator components](simulator_components.md)
- [Simple model](simple-model.md)
- [Download model](download-model.md)
- [Hidden-service model](hidden-service-model.md)
- [Adversaries](adversaries.md)
- [Metrics and formulas](metrics.md)
- [Path selectors](path-selection.md)
- [Results](results.md)
- [References and related work](references.md)
