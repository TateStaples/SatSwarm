# SatSwarm - SAT Problem Simulator

SatSwarm is a custom architecture simulator designed to solve Boolean Satisfiability (SAT) problems using a distributed network of nodes. The simulator implements various network topologies and provides performance metrics for solving SAT problems.

## Architecture Overview

### Main Components

1. **SatSwarm** (`src/structures/satswarm.rs`)
   - Core simulator that manages the network of nodes
   - Handles message passing and clock synchronization
   - Implements different network topologies (Grid, Torus, Dense)
   - Tracks simulation metrics (busy cycles, idle cycles)

2. **Node** (`src/structures/node.rs`)
   - Represents a processing node in the network
   - Maintains local state and processes messages
   - Implements the SAT solving logic
   - Manages neighbor connections

3. **ClauseTable** (`src/structures/clause_table.rs`)
   - Stores and manages the SAT problem clauses
   - Handles loading SAT problems from files
   - Tracks variable assignments and clause states

4. **Message System** (`src/structures/message.rs`)
   - Implements message passing between nodes
   - Handles message queuing and delivery
   - Supports different message types and destinations

### Key Data Structures

1. **TestConfig**
   ```rust
   pub struct TestConfig {
       pub num_nodes: usize,
       pub topology: Topology,
       pub node_bandwidth: usize,
       pub num_vars: usize,
       pub test_dir: String,
   }
   ```
   - Configuration for test runs
   - Specifies network size, topology, and problem parameters

2. **Topology**
   ```rust
   pub enum Topology {
       Grid(usize, usize),
       Torus(usize, usize),
       Dense(usize),
   }
   ```
   - Defines network topology types
   - Grid: Rectangular grid with fixed dimensions
   - Torus: Grid with wrap-around connections
   - Dense: Fully connected network

3. **TestResult**
   ```rust
   pub struct TestResult {
       pub simulated_result: bool,
       pub simulated_cycles: u64,
       pub cycles_busy: u64,
       pub cycles_idle: u64,
   }
   ```
   - Stores simulation results and performance metrics

## Usage

The simulator can be run with various command-line arguments:

```bash
cargo run -- [OPTIONS]
```

Options:
- `--num_nodes <NUM>`: Number of nodes (default: 100)
- `--topology <TOPOLOGY>`: Network topology (default: grid)
- `--test_path <PATH>`: Path to test files (default: tests)
- `--node_bandwidth <BW>`: Node bandwidth (default: 100)
- `--num_vars <NUM>`: Number of variables (default: 50)

## Simulation Process

1. **Initialization**
   - Load SAT problem from file
   - Create network with specified topology
   - Initialize nodes with problem data

2. **Execution**
   - Nodes process messages and update their state
   - Messages are passed between nodes based on topology
   - Simulation continues until solution is found or timeout

3. **Results**
   - Success/failure of solving the SAT problem
   - Performance metrics (cycles, busy/idle time)
   - Comparison with MiniSat solver results

## Performance Metrics

The simulator tracks several performance metrics:
- Total simulation cycles
- Busy cycles (nodes actively processing)
- Idle cycles (nodes waiting for messages)
- Comparison with MiniSat solver performance

## File Structure

- `src/main.rs`: Entry point and command-line interface
- `src/structures/`
  - `satswarm.rs`: Core simulator implementation
  - `node.rs`: Node implementation
  - `clause_table.rs`: SAT problem representation
  - `message.rs`: Message passing system
  - `minisat.rs`: MiniSat solver integration
  - `util_types.rs`: Common type definitions

## Testing

The simulator includes a testing framework that:
- Loads SAT problems from test files
- Compares results with MiniSat solver
- Generates performance logs
- Validates solution correctness
