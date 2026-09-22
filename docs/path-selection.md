# Path selectors

### Session path selectors

| CLI mode | Selection rule                                                                                                                             |
| --- |--------------------------------------------------------------------------------------------------------------------------------------------|
| `random` | Sample a fresh uniform path.                                                                                                               |
| `k-hf` | Choose fixed positions and one node per position once. Resample remaining positions for each request.                                      |
| `k-w` | Create candidate pools at selected positions. Choose uniformly from each pool, then fill fresh positions excluding already selected nodes. |
| `alpha-sticky` | Introduce a first path. Later, with probability alpha choose uniformly from the previously used paths, otherwise add a new path.|

## Time-based path selectors

### Fixed-paths
[TODO ...]

### Fixed topology 
[TODO ...]

### Active paths over a fixed topology (`FPOFT`)
[TODO ...]