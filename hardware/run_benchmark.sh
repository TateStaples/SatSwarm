#!/bin/bash

# SystemVerilog Node Benchmarking Script
# This script runs various tests and collects performance metrics

set -e

echo "=========================================="
echo "SystemVerilog Node Benchmarking Suite"
echo "=========================================="
echo ""

# Create output directory
OUTPUT_DIR="benchmark_results"
mkdir -p "$OUTPUT_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULT_FILE="$OUTPUT_DIR/benchmark_$TIMESTAMP.txt"

# Function to run a test
run_test() {
    local test_name=$1
    local verilog_files=$2
    local simulator=$3
    
    echo "Running $test_name..."
    
    case $simulator in
        "iverilog")
            # Compile with Icarus Verilog
            iverilog -g2012 -o "${OUTPUT_DIR}/node_test" $verilog_files
            if [ $? -ne 0 ]; then
                echo "ERROR: Compilation failed for $test_name"
                return 1
            fi
            
            # Run simulation
            vvp "${OUTPUT_DIR}/node_test" | tee -a "$RESULT_FILE"
            ;;
            
        "verilator")
            # Note: Verilator requires a C++ testbench wrapper
            # This is a simplified command - for full Verilator support,
            # you would need to create a C++ driver file
            echo "NOTE: Verilator support requires additional C++ wrapper"
            echo "Using Icarus Verilog instead..."
            # Fallback to iverilog
            iverilog -g2012 -o "${OUTPUT_DIR}/node_test" $verilog_files
            if [ $? -ne 0 ]; then
                echo "ERROR: Compilation failed for $test_name"
                return 1
            fi
            vvp "${OUTPUT_DIR}/node_test" | tee -a "$RESULT_FILE"
            ;;
            
        *)
            echo "ERROR: Unknown simulator: $simulator"
            return 1
            ;;
    esac
    
    return 0
}

# Check for available simulators
SIMULATOR="iverilog"
if command -v iverilog &> /dev/null; then
    echo "Using Icarus Verilog simulator"
    SIMULATOR="iverilog"
elif command -v verilator &> /dev/null; then
    echo "Using Verilator simulator"
    SIMULATOR="verilator"
else
    echo "ERROR: No suitable simulator found (iverilog or verilator)"
    echo "Please install one of them to run benchmarks"
    exit 1
fi

echo "" | tee "$RESULT_FILE"
echo "=========================================" | tee -a "$RESULT_FILE"
echo "Benchmark Run: $TIMESTAMP" | tee -a "$RESULT_FILE"
echo "Simulator: $SIMULATOR" | tee -a "$RESULT_FILE"
echo "=========================================" | tee -a "$RESULT_FILE"
echo "" | tee -a "$RESULT_FILE"

# Run the main testbench
echo "Running comprehensive testbench..."
run_test "Node SystemVerilog Testbench" "node.sv node_tb.sv" "$SIMULATOR"

# Clean up temporary files
if [ "$SIMULATOR" = "iverilog" ]; then
    rm -f "${OUTPUT_DIR}/node_test"
fi
rm -f node_tb.vcd

echo ""
echo "=========================================="
echo "Benchmark complete!"
echo "Results saved to: $RESULT_FILE"
echo "=========================================="

# Parse and display summary if file exists
if [ -f "$RESULT_FILE" ]; then
    echo ""
    echo "Quick Summary:"
    echo "----------------------------------------"
    grep -E "(Total tests|Passed|Failed|Pass rate)" "$RESULT_FILE" || echo "No summary found"
    echo ""
    echo "Performance Overview:"
    echo "----------------------------------------"
    grep -A 20 "Performance Results:" "$RESULT_FILE" | head -25 || echo "No performance data found"
fi

exit 0
