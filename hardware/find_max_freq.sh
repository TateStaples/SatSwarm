#!/bin/bash

# Exit on error
set -e

# Create a temporary directory for intermediate files
TEMP_DIR="temp_analysis"
mkdir -p $TEMP_DIR

# Step 1: Synthesize the design using Yosys
echo "Synthesizing design..."
yosys -p "
read_verilog node.v
hierarchy -check
synth -top node
dfflibmap -liberty process_lib/sky130_fd_sc_hd__ff_n40C_1v65.lib
abc -liberty process_lib/sky130_fd_sc_hd__ff_n40C_1v65.lib
write_verilog $TEMP_DIR/synthesized.v
" > $TEMP_DIR/synthesis.log

# Step 2: Create OpenSTA script for timing analysis
echo "Creating timing analysis script..."
cat > $TEMP_DIR/find_freq.tcl << EOF
# Read the liberty file
read_liberty process_lib/sky130_fd_sc_hd__ff_n40C_1v65.lib

# Read the synthesized netlist
read_verilog $TEMP_DIR/synthesized.v

# Link the design
link_design node

# Create a clock with a placeholder period
create_clock -name clk -period 1.0 [get_ports clk]
set_clock_uncertainty 0.1 [get_clocks clk]

# Set input and output delay
set_input_delay -clock clk -max 3.0 [all_inputs]
set_input_delay -clock clk -min 0.2 [all_inputs]
set_output_delay -clock clk -max 3.0 [all_outputs]
set_output_delay -clock clk -min 0.3 [all_outputs]

# Set load capacitance
set_load 0.05 [all_outputs]

# Report timing
report_checks -path_delay min_max
report_clock_properties

exit
EOF

# Step 3: Run OpenSTA
echo "Running timing analysis..."
./external/OpenSTA/app/sta $TEMP_DIR/find_freq.tcl  > $TEMP_DIR/timing.log 2>&1

# Display the timing log
echo "Timing analysis results:"
cat $TEMP_DIR/timing.log

# Cleanup
# rm -rf $TEMP_DIR  # Uncomment to clean up temporary files
