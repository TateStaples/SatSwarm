#!/bin/bash

# Create a directory for synthesis results
mkdir -p synthesis_results

# Run Yosys synthesis
echo "Running Yosys synthesis..."
yosys -s synthesize_node.ys

# Run timing analysis using OpenSTA
echo "Running timing analysis..."
sta << EOF
read_liberty process_lib/sky130_fd_sc_hd__ff_n40C_1v65.lib
read_verilog synthesis_results/node_synth.v
link_design node
create_clock -period 10 -name clk
set_input_delay -clock clk 0 [all_inputs]
set_output_delay -clock clk 0 [all_outputs]
report_timing
report_timing_summary
exit
EOF

# Copy synthesis report to results directory
cp synthesis_report.txt synthesis_results/

echo "Synthesis and timing analysis complete. Results are in synthesis_results/" 