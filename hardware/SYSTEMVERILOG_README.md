# SystemVerilog Implementation with Benchmarking

This directory contains a SystemVerilog implementation of the SAT solver node with comprehensive benchmarking capabilities.

## Overview

The SystemVerilog implementation enhances the original Verilog design with:
- Modern SystemVerilog features (logic types, enums, assertions)
- Performance monitoring counters
- Comprehensive testbench with multiple test scenarios
- Automated benchmarking infrastructure

## Files

### Design Files
- **node.sv** - SystemVerilog implementation of the SAT solver node
  - Uses `logic` types instead of `wire`/`reg`
  - Employs `enum` for state machine and message types
  - Includes assertions for design verification
  - Adds performance counters (busy cycles, idle cycles, messages processed)

### Testbench Files
- **node_tb.sv** - Comprehensive SystemVerilog testbench
  - 9 different test scenarios covering various use cases
  - Automated benchmarking and performance measurement
  - Coverage of edge cases and stress testing
  - Detailed performance metrics collection

### Scripts
- **run_benchmark.sh** - Automated benchmark runner
  - Supports multiple simulators (Icarus Verilog, Verilator)
  - Generates timestamped benchmark reports
  - Provides summary statistics

## SystemVerilog Features Used

### 1. Logic Data Types
Instead of traditional `wire` and `reg`, SystemVerilog's `logic` type is used throughout:
```systemverilog
logic clk, rst_n;
logic [7:0] incoming_var;
```

### 2. Enumerated Types
Message types and states are defined using enums for better readability:
```systemverilog
typedef enum logic [1:0] {
    MSG_NONE = 2'b00,
    MSG_FORK = 2'b01,
    MSG_SUBSTITUTION_MASK = 2'b10
} msg_type_e;
```

### 3. Always_ff and Always_comb
Modern SystemVerilog procedural blocks:
```systemverilog
always_ff @(posedge clk or negedge rst_n) begin
    // Sequential logic
end

always_comb begin
    // Combinational logic
end
```

### 4. Assertions
Built-in verification using SystemVerilog assertions:
```systemverilog
assert property (valid_msg_type) 
    else $error("Invalid message type when valid is high");
```

### 5. Enhanced Data Structures
Using structs for organizing benchmark data:
```systemverilog
typedef struct {
    string test_name;
    int cycles;
    real utilization;
} perf_metrics_t;
```

## Module Parameters

The node module is parameterized for flexibility:
- `VAR_WIDTH` (default: 8) - Width of variable identifiers
- `MASK_WIDTH` (default: 3) - Width of substitution mask
- `MAX_CLAUSES` (default: 16) - Maximum number of clauses to process

## Performance Monitoring

The design includes built-in performance counters:
- **cycles_busy** - Number of cycles spent in processing state
- **cycles_idle** - Number of cycles spent waiting for work
- **messages_processed** - Total number of messages handled
- **utilization** - Percentage of time spent doing useful work

## Running Benchmarks

### Prerequisites
Install one of the following simulators:
- Icarus Verilog: `sudo apt-get install iverilog`
- Verilator: `sudo apt-get install verilator`

### Basic Usage
```bash
cd hardware
./run_benchmark.sh
```

The script will:
1. Detect available simulator
2. Compile the design and testbench
3. Run all test scenarios
4. Generate performance reports
5. Save results to `benchmark_results/benchmark_<timestamp>.txt`

### Manual Simulation
You can also run simulations manually:

#### Using Icarus Verilog
```bash
iverilog -g2012 -o node_test node.sv node_tb.sv
vvp node_test
```

#### Using Verilator
```bash
verilator --cc --exe --build node.sv node_tb.sv --trace
./obj_dir/Vnode_tb
```

## Test Scenarios

The testbench includes the following test cases:

1. **Reset Test** - Verifies proper initialization
2. **Single Fork** - Tests basic fork message handling
3. **Complete Processing** - Full clause processing cycle
4. **Multiple Variables** - Sequential variable processing
5. **Various Masks** - Different substitution mask patterns
6. **Stress Test** - Rapid message injection
7. **Max Variable** - Edge case with maximum variable value
8. **Zero Variable** - Edge case with zero variable
9. **Throughput Test** - High-volume processing test

## Benchmark Metrics

For each test, the following metrics are collected:
- **Total Cycles** - Duration of the test in clock cycles
- **Busy Cycles** - Cycles spent actively processing
- **Idle Cycles** - Cycles spent waiting
- **Messages Processed** - Number of messages handled
- **Utilization** - Busy cycles / Total cycles (percentage)

## Example Output

```
========================================
BENCHMARK SUMMARY
========================================
Total tests: 15
Passed: 15
Failed: 0
Pass rate: 100.0%

Performance Results:
Test Name                      Cycles      Busy      Idle  Messages     Util%
-------------------------------------------------------------------------------------
Reset Test                         10         0        10         0       0.00
Single Fork                        15         5        10         1      33.33
Complete Processing                50        35        15        17      70.00
Multiple Variables                150       120        30        51      80.00
...
========================================
```

## Integration with Existing Hardware

The SystemVerilog implementation is designed to be compatible with existing synthesis flows:
- Can be synthesized using the existing `synthesize_node.ys` script
- Compatible with Sky130 PDK library
- Assertions are automatically disabled during synthesis (`ifndef SYNTHESIS`)

## Future Enhancements

Potential improvements to the SystemVerilog implementation:
- Add SystemVerilog interfaces for cleaner module connections
- Implement coverage collection for functional coverage
- Add randomized testing with constraints
- Create a UVM-based testbench for more advanced verification
- Add formal verification properties

## Troubleshooting

### Simulation Issues
- **Compilation errors**: Ensure you're using a SystemVerilog-compatible simulator
- **Missing waveforms**: Check that VCD dumping is enabled in the testbench
- **Timeout errors**: Increase `MAX_CYCLES` parameter in the testbench

### Performance Issues
- If utilization is low, consider adjusting the message injection rate
- For stress testing, modify the loop iterations in test scenarios
- Check timing constraints if synthesis is failing

## References

- SystemVerilog IEEE Standard 1800-2017
- Icarus Verilog Documentation: http://iverilog.icarus.com/
- Verilator Documentation: https://verilator.org/guide/latest/
