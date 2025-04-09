`timescale 1ns/1ps

module node_tb();

    // Parameters
    parameter NODE_ID = 0;
    parameter NUM_NEIGHBORS = 4;
    parameter CLAUSE_LENGTH = 3;
    parameter NUM_CLAUSES = 16;
    parameter VAR_WIDTH = 8;
    
    // Clock and reset
    reg clk;
    reg rst_n;
    
    // Node interface
    reg [VAR_WIDTH-1:0] incoming_var;
    reg incoming_var_valid;
    reg [1:0] incoming_msg_type;
    reg [CLAUSE_LENGTH-1:0] incoming_mask;
    reg [NUM_NEIGHBORS-1:0] neighbor_busy;
    
    // Output interface
    wire [VAR_WIDTH-1:0] outgoing_var;
    wire outgoing_var_valid;
    wire [1:0] outgoing_msg_type;
    wire [CLAUSE_LENGTH-1:0] outgoing_mask;
    wire [NUM_NEIGHBORS-1:0] node_busy;
    wire sat_found;
    
    // Instantiate the node
    node #(
        .NODE_ID(NODE_ID),
        .NUM_NEIGHBORS(NUM_NEIGHBORS),
        .CLAUSE_LENGTH(CLAUSE_LENGTH),
        .NUM_CLAUSES(NUM_CLAUSES),
        .VAR_WIDTH(VAR_WIDTH)
    ) dut (
        .clk(clk),
        .rst_n(rst_n),
        .incoming_var(incoming_var),
        .incoming_var_valid(incoming_var_valid),
        .incoming_msg_type(incoming_msg_type),
        .incoming_mask(incoming_mask),
        .neighbor_busy(neighbor_busy),
        .outgoing_var(outgoing_var),
        .outgoing_var_valid(outgoing_var_valid),
        .outgoing_msg_type(outgoing_msg_type),
        .outgoing_mask(outgoing_mask),
        .node_busy(node_busy),
        .sat_found(sat_found)
    );
    
    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk; // 100MHz clock
    end
    
    // Test stimulus
    initial begin
        // Initialize inputs
        rst_n = 0;
        incoming_var = 0;
        incoming_var_valid = 0;
        incoming_msg_type = 0;
        incoming_mask = 0;
        neighbor_busy = 0;
        
        // Reset
        #20;
        rst_n = 1;
        
        // Test case 1: Send a fork message
        #10;
        incoming_var = 8'h42;
        incoming_var_valid = 1;
        incoming_msg_type = 2'b01; // MSG_FORK
        
        #10;
        incoming_var_valid = 0;
        
        // Test case 2: Process a substitution mask
        #20;
        incoming_msg_type = 2'b10; // MSG_SUBSTITUTION_MASK
        incoming_mask = 3'b101; // One term true, one false, one symbolic
        
        #10;
        incoming_msg_type = 0;
        
        // Test case 3: Variable not found
        #30;
        incoming_msg_type = 2'b11; // MSG_VARIABLE_NOT_FOUND
        
        #10;
        incoming_msg_type = 0;
        
        // Test case 4: All neighbors busy
        #20;
        neighbor_busy = 4'b1111;
        
        #100;
        
        // End simulation
        $finish;
    end
    
    // Monitor outputs
    initial begin
        $monitor("Time=%0t rst_n=%b state=%b sat_found=%b", 
                 $time, rst_n, dut.current_state, sat_found);
    end
    
endmodule 