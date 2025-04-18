#!/bin/bash
# Templete
# cargo run --release -- \
#                 --num_nodes $DEFAULT_NODES \
#                 --topology "$DFAULT_TOPOLOGY" \
#                 --test_path "$TEST_PATH" \
#                 --node_bandwidth $DEFAULT_PARALLELISM
#                 --num_vars $DEFAULT_PARALLELISM
# Dataset path
TEST_PATH="tests/satlib"

# Parameter sets
TOPOLOGIES=("grid" "torus" "dense")  # Default torus
NODE_COUNTS=(4 16 64 256 1024)  # Default 64
CLAUSE_PARALLELISMS=(1 10 100 1000)  # Default 100
NUM_VARS=(20 50)

DEFAULT_TOPOLOGY="torus"
DEFAULT_NODES=64
DEFAULT_PARALLELISM=100
DEFAULT_NUM_VARS=50

for TOPOLOGY in "${TOPOLOGIES[@]}"; do
    # NODES = DEFAULT_NODES
    # PARALLELISM = "$DEFAULT_PARALLELISM"
    echo "Running with topology=$TOPOLOGY, nodes=$NODES, node_bandwidth=$PARALLELISM"
            cargo run --release -- \
                --num_nodes $DEFAULT_NODES \
                --topology "$TOPOLOGY" \
                --test_path "$TEST_PATH" \
                --node_bandwidth $DEFAULT_PARALLELISM \
                --num_vars $DEFAULT_NUM_VARS
done

for NODES in "${NODE_COUNTS[@]}"; do
    TOPOLOGY="$DEFAULT_TOPOLOGY"
    PARALLELISM="$DEFAULT_PARALLELISM"
    echo "Running with topology=$TOPOLOGY, nodes=$NODES, node_bandwidth=$PARALLELISM"
            cargo run --release -- \
                --num_nodes $NODES \
                --topology "$TOPOLOGY" \
                --test_path "$TEST_PATH" \
                --node_bandwidth $DEFAULT_PARALLELISM \
                --num_vars $DEFAULT_NUM_VARS
done

for PARALLELISM in "${CLAUSE_PARALLELISMS[@]}"; do
    TOPOLOGY="$DEFAULT_TOPOLOGY"
    NODES="$DEFAULT_NODES"
    echo "Running with topology=$TOPOLOGY, nodes=$NODES, node_bandwidth=$PARALLELISM"
            cargo run --release -- \
                --num_nodes $DEFAULT_NODES \
                --topology "$TOPOLOGY" \
                --test_path "$TEST_PATH" \
                --node_bandwidth $DEFAULT_PARALLELISM \
                --num_vars $DEFAULT_NUM_VARS
done

for NUM_VARS in "${NUM_VARS[@]}"; do
    TOPOLOGY="$DEFAULT_TOPOLOGY"
    NODES="$DEFAULT_NODES"
    PARALLELISM="$DEFAULT_PARALLELISM"
    echo "Running with topology=$TOPOLOGY, nodes=$NODES, node_bandwidth=$PARALLELISM"
            cargo run --release -- \
                --num_nodes $DEFAULT_NODES \
                --topology "$TOPOLOGY" \
                --test_path "$TEST_PATH" \
                --node_bandwidth $DEFAULT_PARALLELISM \
                --num_vars $NUM_VARS
done
