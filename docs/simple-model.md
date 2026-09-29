# Simple message model

The simple model represents one user sending messages over a some period of time. It models exposure to malicious paths over repeated use of the mixnet.

## Traffic behavior

The traffic behavior for the simple model is very simple. The user's clock starts at zero. Before each message, the model samples an independent delay uniformly from `[300, 900)` seconds (5-15 min).

The clock moves by the sampled delay. The model samples a path and asks `SybilAdversary` whether all its nodes are malicious. If the new timestamp exceeds the simulation deadline or the adversary check returns true, the simulation stops.
