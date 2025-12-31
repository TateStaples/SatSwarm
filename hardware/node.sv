// SystemVerilog implementation of SAT solver node
// Enhanced with SV features: logic types, better state handling, assertions

module node #(
    parameter int VAR_WIDTH = 8,
    parameter int MASK_WIDTH = 3,
    parameter int MAX_CLAUSES = 16
) (
    input  logic clk,
    input  logic rst_n,
    
    // Input interface
    input  logic [VAR_WIDTH-1:0] incoming_var,
    input  logic incoming_var_valid,
    input  logic [1:0] incoming_msg_type,  // 00: None, 01: Fork, 10: SubstitutionMask
    input  logic [MASK_WIDTH-1:0] incoming_mask,
    
    // Output interface
    output logic [VAR_WIDTH-1:0] outgoing_var,
    output logic outgoing_var_valid,
    output logic [1:0] outgoing_msg_type,
    output logic [MASK_WIDTH-1:0] outgoing_mask,
    output logic node_busy,
    output logic sat_found,
    
    // Performance monitoring outputs
    output logic [31:0] cycles_busy,
    output logic [31:0] cycles_idle,
    output logic [31:0] messages_processed
);

    // Message type encoding using enum for better readability
    typedef enum logic [1:0] {
        MSG_NONE = 2'b00,
        MSG_FORK = 2'b01,
        MSG_SUBSTITUTION_MASK = 2'b10
    } msg_type_e;
    
    // State machine encoding
    typedef enum logic [1:0] {
        IDLE = 2'b00,
        PROCESSING = 2'b01,
        DONE = 2'b10
    } state_e;
    
    // Internal registers
    state_e current_state, next_state;
    logic [VAR_WIDTH-1:0] var_counter;
    logic [$clog2(MAX_CLAUSES):0] clause_counter;
    
    // Performance counters
    logic [31:0] busy_counter;
    logic [31:0] idle_counter;
    logic [31:0] msg_counter;
    
    // Assign outputs from internal counters
    assign cycles_busy = busy_counter;
    assign cycles_idle = idle_counter;
    assign messages_processed = msg_counter;
    
    // State machine - sequential logic
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= IDLE;
            var_counter <= '0;
            clause_counter <= '0;
            outgoing_var <= '0;
            outgoing_var_valid <= 1'b0;
            outgoing_msg_type <= MSG_NONE;
            outgoing_mask <= '0;
            node_busy <= 1'b0;
            sat_found <= 1'b0;
            busy_counter <= '0;
            idle_counter <= '0;
            msg_counter <= '0;
        end else begin
            current_state <= next_state;
            
            // Default values
            outgoing_var_valid <= 1'b0;
            outgoing_msg_type <= MSG_NONE;
            
            // Update performance counters
            if (node_busy)
                busy_counter <= busy_counter + 1;
            else
                idle_counter <= idle_counter + 1;
            
            case (current_state)
                IDLE: begin
                    node_busy <= 1'b0;
                    if (incoming_var_valid && incoming_msg_type == MSG_FORK) begin
                        var_counter <= incoming_var;
                        clause_counter <= '0;
                        msg_counter <= msg_counter + 1;
                    end
                end
                
                PROCESSING: begin
                    node_busy <= 1'b1;
                    
                    if (incoming_msg_type == MSG_SUBSTITUTION_MASK) begin
                        outgoing_var <= var_counter;
                        outgoing_var_valid <= 1'b1;
                        outgoing_msg_type <= MSG_SUBSTITUTION_MASK;
                        outgoing_mask <= incoming_mask;
                        msg_counter <= msg_counter + 1;
                        
                        if (clause_counter == MAX_CLAUSES - 1) begin
                            // Completed processing all clauses - fork to next variable
                            outgoing_var <= var_counter + 1;
                            outgoing_var_valid <= 1'b1;
                            outgoing_msg_type <= MSG_FORK;
                            clause_counter <= '0;
                            
                            // Check if we've reached the maximum variable
                            if (var_counter == {VAR_WIDTH{1'b1}}) begin
                                sat_found <= 1'b1;
                            end
                        end else begin
                            clause_counter <= clause_counter + 1;
                        end
                    end
                end
                
                DONE: begin
                    node_busy <= 1'b0;
                    sat_found <= 1'b1;
                end
                
                default: begin
                    // Should never reach here
                    current_state <= IDLE;
                end
            endcase
        end
    end
    
    // State machine - combinational logic
    always_comb begin
        next_state = current_state;
        
        case (current_state)
            IDLE: begin
                if (incoming_var_valid && incoming_msg_type == MSG_FORK) begin
                    next_state = PROCESSING;
                end
            end
            
            PROCESSING: begin
                if (sat_found) begin
                    next_state = DONE;
                end else if (clause_counter == MAX_CLAUSES - 1 && 
                           incoming_msg_type == MSG_SUBSTITUTION_MASK) begin
                    // After processing all clauses, return to IDLE
                    next_state = IDLE;
                end
            end
            
            DONE: begin
                // Stay in DONE state
                next_state = DONE;
            end
            
            default: begin
                next_state = IDLE;
            end
        endcase
    end
    
    // Assertions for verification (disabled for Icarus Verilog compatibility)
    // Note: Enable these with a simulator that fully supports SystemVerilog assertions
    `ifdef ENABLE_ASSERTIONS
        // Check that valid signals are properly set
        property valid_msg_type;
            @(posedge clk) disable iff (!rst_n)
            outgoing_var_valid |-> (outgoing_msg_type == MSG_FORK || 
                                   outgoing_msg_type == MSG_SUBSTITUTION_MASK);
        endproperty
        assert property (valid_msg_type) 
            else $error("Invalid message type when valid is high");
        
        // Check that we don't overflow clause counter
        property clause_counter_limit;
            @(posedge clk) disable iff (!rst_n)
            clause_counter <= MAX_CLAUSES;
        endproperty
        assert property (clause_counter_limit) 
            else $error("Clause counter exceeded MAX_CLAUSES");
        
        // Check that busy signal matches state
        property busy_matches_state;
            @(posedge clk) disable iff (!rst_n)
            (current_state == PROCESSING) |-> node_busy;
        endproperty
        assert property (busy_matches_state) 
            else $error("Busy signal doesn't match processing state");
    `endif

endmodule
