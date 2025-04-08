#!/bin/bash

# Compile the design and testbench
iverilog -o node_test node.v node_tb.v

# Run the simulation
vvp node_test

# Clean up
rm node_test 