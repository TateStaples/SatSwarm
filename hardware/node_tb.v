`timescale 1ns/1ps

module node_tb;
    // Clock period
    localparam CLK_PERIOD = 10;
    
    // Signals
    reg clk;
    reg rst_n;
    reg [7:0] incoming_var;
    reg incoming_var_valid;
    reg [1:0] incoming_msg_type;
    reg [2:0] incoming_mask;
    
    wire [7:0] outgoing_var;
    wire outgoing_var_valid;
    wire [1:0] outgoing_msg_type;
    wire [2:0] outgoing_mask;
    wire node_busy;
    wire sat_found;
    
    // Message type encoding
    localparam MSG_NONE = 2'b00;
    localparam MSG_FORK = 2'b01;
    localparam MSG_SUBSTITUTION_MASK = 2'b10;
    
    // Instantiate the node
    node dut (
        .clk(clk),
        .rst_n(rst_n),
        .incoming_var(incoming_var),
        .incoming_var_valid(incoming_var_valid),
        .incoming_msg_type(incoming_msg_type),
        .incoming_mask(incoming_mask),
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
        forever #(CLK_PERIOD/2) clk = ~clk;
    end
    
    // Test stimulus
    initial begin
        // Initialize signals
        rst_n = 0;
        incoming_var = 0;
        incoming_var_valid = 0;
        incoming_msg_type = MSG_NONE;
        incoming_mask = 0;
        
        // Reset
        #(CLK_PERIOD * 2);
        rst_n = 1;
        #(CLK_PERIOD);
        
        // Test 1: Send initial fork
        incoming_var = 8'd42;
        incoming_var_valid = 1;
        incoming_msg_type = MSG_FORK;
        #(CLK_PERIOD);
        incoming_var_valid = 0;
        #(CLK_PERIOD);
        
        // Test 2: Process clauses
        repeat(16) begin
            incoming_msg_type = MSG_SUBSTITUTION_MASK;
            incoming_mask = 3'b001;
            #(CLK_PERIOD);
            incoming_msg_type = MSG_NONE;
            #(CLK_PERIOD);
        end
        
        // End simulation
        #(CLK_PERIOD * 4);
        $finish;
    end
    
    // Monitor outputs
    initial begin
        $monitor("Time=%0t state=%b busy=%b sat=%b", 
                 $time, dut.state, node_busy, sat_found);
    end
    
endmodule 