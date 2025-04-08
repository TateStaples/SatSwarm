#!/bin/bash

# Create output directory if it doesn't exist
mkdir -p output

# Create SDC file with timing constraints
cat > output/node.sdc << EOF
# Clock definition
create_clock -name clk -period 10.0 [get_ports clk]
set_clock_uncertainty 0.1 [get_clocks clk]

# Input delays for all synchronous inputs
set_input_delay -clock clk -max 2.0 [all_inputs]
set_input_delay -clock clk -min 0.5 [all_inputs]

# Output delays
set_output_delay -clock clk -max 2.0 [all_outputs]
set_output_delay -clock clk -min 0.67 [all_outputs]

# Load capacitance for all outputs
set_load 0.1 [all_outputs]
EOF

# Run Yosys synthesis
echo "Running Yosys synthesis..."
yosys synthesize_node.ys

# Check if synthesis was successful
if [ $? -eq 0 ]; then
    echo "Synthesis completed successfully. Output written to node_synth.v"
else
    echo "Synthesis failed!"
    exit 1
fi

# Run timing analysis using OpenSTA
echo "Running timing analysis with OpenSTA..."
./external/OpenSTA/app/sta -f output/node.sdc -d node_synth.v

# Check if timing analysis was successful
if [ $? -eq 0 ]; then
    echo "Timing analysis completed successfully."
else
    echo "Timing analysis failed!"
    exit 1
fi

echo "Synthesis and timing analysis completed." 