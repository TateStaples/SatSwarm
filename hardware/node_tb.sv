// SystemVerilog testbench for node with comprehensive benchmarking
`timescale 1ns/1ps

module node_tb;
    // Clock parameters
    localparam CLK_PERIOD = 10;
    localparam int MAX_CYCLES = 10000;
    
    // Module parameters
    localparam int VAR_WIDTH = 8;
    localparam int MASK_WIDTH = 3;
    localparam int MAX_CLAUSES = 16;
    
    // Signals
    logic clk;
    logic rst_n;
    logic [VAR_WIDTH-1:0] incoming_var;
    logic incoming_var_valid;
    logic [1:0] incoming_msg_type;
    logic [MASK_WIDTH-1:0] incoming_mask;
    
    logic [VAR_WIDTH-1:0] outgoing_var;
    logic outgoing_var_valid;
    logic [1:0] outgoing_msg_type;
    logic [MASK_WIDTH-1:0] outgoing_mask;
    logic node_busy;
    logic sat_found;
    
    logic [31:0] cycles_busy;
    logic [31:0] cycles_idle;
    logic [31:0] messages_processed;
    
    // Message type encoding
    typedef enum logic [1:0] {
        MSG_NONE = 2'b00,
        MSG_FORK = 2'b01,
        MSG_SUBSTITUTION_MASK = 2'b10
    } msg_type_e;
    
    // Benchmarking variables
    int test_count = 0;
    int pass_count = 0;
    int fail_count = 0;
    longint start_time;
    longint end_time;
    
    // Performance metrics arrays (simplified for compatibility)
    string perf_test_names [20];
    int perf_cycles [20];
    int perf_busy [20];
    int perf_idle [20];
    int perf_messages [20];
    real perf_utilization [20];
    int perf_count = 0;
    
    // Instantiate the DUT
    node #(
        .VAR_WIDTH(VAR_WIDTH),
        .MASK_WIDTH(MASK_WIDTH),
        .MAX_CLAUSES(MAX_CLAUSES)
    ) dut (
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
        .sat_found(sat_found),
        .cycles_busy(cycles_busy),
        .cycles_idle(cycles_idle),
        .messages_processed(messages_processed)
    );
    
    // Clock generation
    initial begin
        clk = 0;
        forever #(CLK_PERIOD/2) clk = ~clk;
    end
    
    // Watchdog timer
    initial begin
        #(CLK_PERIOD * MAX_CYCLES);
        $display("ERROR: Simulation timeout after %0d cycles", MAX_CYCLES);
        print_benchmark_summary();
        $finish;
    end
    
    // Task to reset the DUT
    task automatic reset_dut();
        rst_n = 0;
        incoming_var = 0;
        incoming_var_valid = 0;
        incoming_msg_type = MSG_NONE;
        incoming_mask = 0;
        repeat(3) @(posedge clk);
        rst_n = 1;
        @(posedge clk);
    endtask
    
    // Task to send a fork message
    task automatic send_fork(input logic [VAR_WIDTH-1:0] var_id);
        @(posedge clk);
        incoming_var = var_id;
        incoming_var_valid = 1;
        incoming_msg_type = MSG_FORK;
        @(posedge clk);
        incoming_var_valid = 0;
        incoming_msg_type = MSG_NONE;
    endtask
    
    // Task to send a substitution mask message
    task automatic send_substitution(input logic [MASK_WIDTH-1:0] mask);
        @(posedge clk);
        incoming_msg_type = MSG_SUBSTITUTION_MASK;
        incoming_mask = mask;
        @(posedge clk);
        incoming_msg_type = MSG_NONE;
    endtask
    
    // Task to wait for node to become idle
    task automatic wait_idle(input int max_wait = 100);
        int wait_count = 0;
        while (node_busy && wait_count < max_wait) begin
            @(posedge clk);
            wait_count++;
        end
        if (wait_count >= max_wait) begin
            $display("WARNING: Timeout waiting for idle");
        end
    endtask
    
    // Task to start performance measurement
    task automatic start_perf_measurement();
        start_time = $time;
    endtask
    
    // Task to end performance measurement and record results
    task automatic end_perf_measurement(input string test_name);
        int idx;
        int total_cycles;
        idx = perf_count;
        end_time = $time;
        
        total_cycles = (end_time - start_time) / CLK_PERIOD;
        
        perf_test_names[idx] = test_name;
        perf_cycles[idx] = total_cycles;
        perf_busy[idx] = cycles_busy;
        perf_idle[idx] = cycles_idle;
        perf_messages[idx] = messages_processed;
        perf_utilization[idx] = real'(cycles_busy) / real'(total_cycles) * 100.0;
        
        perf_count = perf_count + 1;
        
        $display("Performance metrics for %s:", test_name);
        $display("  Total cycles: %0d", total_cycles);
        $display("  Busy cycles: %0d", perf_busy[idx]);
        $display("  Idle cycles: %0d", perf_idle[idx]);
        $display("  Messages processed: %0d", perf_messages[idx]);
        $display("  Utilization: %0.2f%%", perf_utilization[idx]);
    endtask
    
    // Task to check result
    task automatic check_result(input string test_name, input logic expected, input logic actual);
        test_count++;
        if (expected === actual) begin
            $display("PASS: %s", test_name);
            pass_count++;
        end else begin
            $display("FAIL: %s - Expected: %b, Got: %b", test_name, expected, actual);
            fail_count++;
        end
    endtask
    
    // Task to print benchmark summary
    task automatic print_benchmark_summary();
        int i;
        $display("\n========================================");
        $display("BENCHMARK SUMMARY");
        $display("========================================");
        $display("Total tests: %0d", test_count);
        $display("Passed: %0d", pass_count);
        $display("Failed: %0d", fail_count);
        $display("Pass rate: %0.1f%%", real'(pass_count) / real'(test_count) * 100.0);
        $display("\nPerformance Results:");
        $display("%-30s %10s %10s %10s %10s %10s", 
                 "Test Name", "Cycles", "Busy", "Idle", "Messages", "Util%");
        $display("-------------------------------------------------------------------------------------");
        for (i = 0; i < perf_count; i = i + 1) begin
            $display("%-30s %10d %10d %10d %10d %10.2f", 
                     perf_test_names[i],
                     perf_cycles[i],
                     perf_busy[i],
                     perf_idle[i],
                     perf_messages[i],
                     perf_utilization[i]);
        end
        $display("========================================\n");
    endtask
    
    // Main test sequence
    initial begin
        $display("========================================");
        $display("Starting SystemVerilog Node Testbench");
        $display("========================================\n");
        
        // Test 1: Basic reset test
        $display("\n--- Test 1: Basic Reset ---");
        start_perf_measurement();
        reset_dut();
        check_result("Node not busy after reset", 1'b0, node_busy);
        check_result("SAT not found after reset", 1'b0, sat_found);
        repeat(5) @(posedge clk);
        end_perf_measurement("Reset Test");
        
        // Test 2: Single fork message
        $display("\n--- Test 2: Single Fork Message ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'd42);
        @(posedge clk);
        check_result("Node busy after fork", 1'b1, node_busy);
        repeat(5) @(posedge clk);
        end_perf_measurement("Single Fork");
        
        // Test 3: Complete clause processing
        $display("\n--- Test 3: Complete Clause Processing ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'd10);
        repeat(MAX_CLAUSES) begin
            send_substitution(3'b001);
        end
        wait_idle(50);
        check_result("Node idle after processing", 1'b0, node_busy);
        end_perf_measurement("Complete Processing");
        
        // Test 4: Multiple variable processing
        $display("\n--- Test 4: Multiple Variables ---");
        reset_dut();
        start_perf_measurement();
        for (int i = 0; i < 3; i++) begin
            send_fork(8'd20 + i);
            repeat(MAX_CLAUSES) begin
                send_substitution(3'b010);
            end
            wait_idle(50);
        end
        end_perf_measurement("Multiple Variables");
        
        // Test 5: Different mask patterns
        $display("\n--- Test 5: Various Mask Patterns ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'd30);
        for (int i = 0; i < MAX_CLAUSES; i++) begin
            send_substitution(i[MASK_WIDTH-1:0]);
        end
        wait_idle(50);
        end_perf_measurement("Various Masks");
        
        // Test 6: Stress test - rapid messages
        $display("\n--- Test 6: Stress Test ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'd50);
        for (int i = 0; i < MAX_CLAUSES * 3; i++) begin
            send_substitution(3'b111);
        end
        wait_idle(200);
        end_perf_measurement("Stress Test");
        
        // Test 7: Maximum variable test
        $display("\n--- Test 7: Maximum Variable ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'hFF); // Maximum variable value
        repeat(MAX_CLAUSES) begin
            send_substitution(3'b001);
        end
        wait_idle(50);
        check_result("SAT found at max variable", 1'b1, sat_found);
        end_perf_measurement("Max Variable");
        
        // Test 8: Edge case - zero variable
        $display("\n--- Test 8: Zero Variable ---");
        reset_dut();
        start_perf_measurement();
        send_fork(8'd0);
        repeat(MAX_CLAUSES) begin
            send_substitution(3'b000);
        end
        wait_idle(50);
        end_perf_measurement("Zero Variable");
        
        // Test 9: Performance test - throughput
        $display("\n--- Test 9: Throughput Test ---");
        reset_dut();
        start_perf_measurement();
        begin
            logic [MASK_WIDTH-1:0] mask_val;
            for (int v = 0; v < 10; v++) begin
                mask_val = (v % 8);
                send_fork(v);
                repeat(MAX_CLAUSES) begin
                    send_substitution(mask_val);
                end
            end
        end
        wait_idle(500);
        end_perf_measurement("Throughput Test");
        
        // Print final summary
        repeat(10) @(posedge clk);
        print_benchmark_summary();
        
        if (fail_count == 0) begin
            $display("ALL TESTS PASSED!");
        end else begin
            $display("SOME TESTS FAILED!");
        end
        
        $finish;
    end
    
    // Waveform dumping
    initial begin
        $dumpfile("node_tb.vcd");
        $dumpvars(0, node_tb);
    end
    
    // Monitor for debugging
    initial begin
        $monitor("Time=%0t rst_n=%b state=%b busy=%b sat=%b out_valid=%b msg_type=%b", 
                 $time, rst_n, dut.current_state, node_busy, sat_found, 
                 outgoing_var_valid, outgoing_msg_type);
    end

endmodule
