# Event-driven hidden-service model

This model studies how long a service resists an adversary that observes its current paths and tries to gain control of mix nodes used on fixed paths.

## Simulation events
At time zero, the model will construct and schedule initial events for the path sampler and the adversary walker.

Later the model will do the following:

1. Take the next event
2. move time directly to that timestamp.
3. Deliver the event to the sampler or adversary that owns it so it can handle/process that event.
4. check for a win
5. if no win, collect new events from sampler and adversary and add them to the queue
6. Return the current timestamp and outcome.

The model stops at the first win, when the queue becomes empty, or when its next event exceeds the simulation deadline (the hidden service lifetime). 

[TODO: add more details...]
