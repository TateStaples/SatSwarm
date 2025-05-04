# Dataset
- all 50 variable problems tests/satlib

# Types
- Topology (grid, torus, dense, twisted)
- \# of nodes (4, 16, 64, 256, 1024)
- Clause parallelism (1, 10, 100, 1000)

## Bash Script
```rust
println!("Usage: cargo run -- [OPTIONS]");
println!("Options:");
println!("  --num_nodes <NUM>       Number of nodes (default: 100)");
println!("  --topology <TOPOLOGY>   Topology (default: grid)");
println!("  --test_path <PATH>      Path to test files (default: tests)");
println!("  --node_bandwidth <BW>   Node bandwidth (default: 100)");
```
cargo run --release -- --num_nodes [nodes] --topology grid --test_path tests/satlib --node_bandwidth [nodes]
```bash
#!/bin/bash

# Dataset path
TEST_PATH="tests/satlib"

# Parameter sets
TOPOLOGIES=("grid" "torus" "dense")
NODE_COUNTS=(4 16 64 256 1024)
CLAUSE_PARALLELISMS=(1 10 100 1000)

# Loop over all combinations
for TOPOLOGY in "${TOPOLOGIES[@]}"; do
    for NODES in "${NODE_COUNTS[@]}"; do
        for PARALLELISM in "${CLAUSE_PARALLELISMS[@]}"; do
            echo "Running with topology=$TOPOLOGY, nodes=$NODES, node_bandwidth=$PARALLELISM"
            cargo run --release -- \
                --num_nodes "$NODES" \
                --topology "$TOPOLOGY" \
                --test_path "$TEST_PATH" \
                --node_bandwidth "$NODES" \
                --clause_parallelism "$PARALLELISM"
        done
    done
done
```


