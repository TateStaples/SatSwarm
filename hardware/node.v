module node #(
    parameter NODE_ID = 0,
    parameter NUM_NEIGHBORS = 4,
    parameter CLAUSE_LENGTH = 3,
    parameter NUM_CLAUSES = 16,
    parameter VAR_WIDTH = 8
) (
    input wire clk,
    input wire rst_n,
    
    // Node interface
    input wire [VAR_WIDTH-1:0] incoming_var,
    input wire incoming_var_valid,
    input wire [1:0] incoming_msg_type, // 00: None, 01: Fork, 10: SubstitutionMask, 11: VariableNotFound
    input wire [CLAUSE_LENGTH-1:0] incoming_mask,
    input wire [NUM_NEIGHBORS-1:0] neighbor_busy,
    
    // Output interface
    output reg [VAR_WIDTH-1:0] outgoing_var,
    output reg outgoing_var_valid,
    output reg [1:0] outgoing_msg_type,
    output reg [CLAUSE_LENGTH-1:0] outgoing_mask,
    output reg [NUM_NEIGHBORS-1:0] node_busy,
    output reg sat_found
);

    // State encoding
    localparam STATE_AWAITING_FORK = 3'b000;
    localparam STATE_BRANCHING = 3'b001;
    localparam STATE_PROCESSING = 3'b010;
    localparam STATE_RECEIVING_FORK = 3'b011;
    localparam STATE_ABORTING = 3'b100;
    
    // Message type encoding
    localparam MSG_NONE = 2'b00;
    localparam MSG_FORK = 2'b01;
    localparam MSG_SUBSTITUTION_MASK = 2'b10;
    localparam MSG_VARIABLE_NOT_FOUND = 2'b11;
    
    // Term state encoding
    localparam TERM_FALSE = 2'b00;
    localparam TERM_TRUE = 2'b01;
    localparam TERM_SYMBOLIC = 2'b10;
    
    // Internal registers
    reg [2:0] current_state;
    reg [2:0] next_state;
    
    reg [VAR_WIDTH-1:0] last_update;
    reg [7:0] clause_index;
    reg sat_flag;
    
    // CNF state table - simplified representation
    reg [1:0] cnf_state [NUM_CLAUSES-1:0][CLAUSE_LENGTH-1:0];
    
    // Speculative branches - simplified as a single register for timing analysis
    reg [VAR_WIDTH-1:0] speculative_branch;
    reg has_speculative_branch;
    
    // Watchdog counter
    reg [15:0] watchdog_counter;
    reg watchdog_timeout;
    
    // Combinational logic for next state
    always @(*) begin
        next_state = current_state;
        
        case (current_state)
            STATE_AWAITING_FORK: begin
                if (incoming_var_valid && incoming_msg_type == MSG_FORK) begin
                    next_state = STATE_RECEIVING_FORK;
                end
            end
            
            STATE_BRANCHING: begin
                // Find a free neighbor or do speculative branching
                if (!neighbor_busy[0]) begin
                    next_state = STATE_PROCESSING;
                end else if (!neighbor_busy[1]) begin
                    next_state = STATE_PROCESSING;
                end else if (!neighbor_busy[2]) begin
                    next_state = STATE_PROCESSING;
                end else if (!neighbor_busy[3]) begin
                    next_state = STATE_PROCESSING;
                end else begin
                    next_state = STATE_PROCESSING; // Speculative branching
                end
            end
            
            STATE_PROCESSING: begin
                if (incoming_msg_type == MSG_VARIABLE_NOT_FOUND) begin
                    next_state = STATE_AWAITING_FORK;
                end else if (incoming_msg_type == MSG_SUBSTITUTION_MASK) begin
                    if (clause_index == NUM_CLAUSES - 1) begin
                        if (sat_flag) begin
                            next_state = STATE_AWAITING_FORK;
                        end else begin
                            next_state = STATE_BRANCHING;
                        end
                    end
                end
            end
            
            STATE_RECEIVING_FORK: begin
                next_state = STATE_PROCESSING;
            end
            
            STATE_ABORTING: begin
                next_state = STATE_AWAITING_FORK;
            end
            
            default: next_state = STATE_AWAITING_FORK;
        endcase
    end
    
    // Sequential logic
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= STATE_AWAITING_FORK;
            last_update <= 0;
            clause_index <= 0;
            sat_flag <= 1'b1;
            has_speculative_branch <= 1'b0;
            speculative_branch <= 0;
            watchdog_counter <= 0;
            watchdog_timeout <= 1'b0;
            sat_found <= 1'b0;
            
            // Initialize CNF state
            for (integer i = 0; i < NUM_CLAUSES; i = i + 1) begin
                for (integer j = 0; j < CLAUSE_LENGTH; j = j + 1) begin
                    cnf_state[i][j] <= TERM_SYMBOLIC;
                end
            end
            
            // Initialize outputs
            outgoing_var <= 0;
            outgoing_var_valid <= 1'b0;
            outgoing_msg_type <= MSG_NONE;
            outgoing_mask <= 0;
            node_busy <= 0;
        end else begin
            current_state <= next_state;
            
            // Watchdog logic
            if (current_state != STATE_AWAITING_FORK) begin
                if (watchdog_counter == 16'hFFFF) begin
                    watchdog_timeout <= 1'b1;
                    current_state <= STATE_AWAITING_FORK;
                end else begin
                    watchdog_counter <= watchdog_counter + 1;
                end
            end else begin
                watchdog_counter <= 0;
                watchdog_timeout <= 1'b0;
            end
            
            // State-specific logic
            case (current_state)
                STATE_AWAITING_FORK: begin
                    outgoing_var_valid <= 1'b0;
                    outgoing_msg_type <= MSG_NONE;
                    node_busy <= 0;
                end
                
                STATE_BRANCHING: begin
                    outgoing_var <= last_update;
                    outgoing_var_valid <= 1'b1;
                    outgoing_msg_type <= MSG_FORK;
                    node_busy <= 4'b1111; // All neighbors busy
                    
                    // Find a free neighbor
                    if (!neighbor_busy[0]) begin
                        node_busy <= 4'b0001;
                    end else if (!neighbor_busy[1]) begin
                        node_busy <= 4'b0010;
                    end else if (!neighbor_busy[2]) begin
                        node_busy <= 4'b0100;
                    end else if (!neighbor_busy[3]) begin
                        node_busy <= 4'b1000;
                    end else begin
                        // Speculative branching
                        has_speculative_branch <= 1'b1;
                        speculative_branch <= last_update;
                    end
                end
                
                STATE_PROCESSING: begin
                    outgoing_var <= last_update;
                    outgoing_var_valid <= 1'b1;
                    outgoing_msg_type <= MSG_SUBSTITUTION_MASK;
                    node_busy <= 4'b1111;
                    
                    if (incoming_msg_type == MSG_SUBSTITUTION_MASK) begin
                        // Process clause
                        for (integer i = 0; i < CLAUSE_LENGTH; i = i + 1) begin
                            if (incoming_mask[i] == 2'b01) begin // True
                                cnf_state[clause_index][i] <= TERM_TRUE;
                            end else if (incoming_mask[i] == 2'b00) begin // False
                                cnf_state[clause_index][i] <= TERM_FALSE;
                            end
                        end
                        
                        // Check if clause is satisfied
                        sat_flag <= 1'b0;
                        for (integer i = 0; i < CLAUSE_LENGTH; i = i + 1) begin
                            if (cnf_state[clause_index][i] == TERM_TRUE) begin
                                sat_flag <= 1'b1;
                            end
                        end
                        
                        clause_index <= clause_index + 1;
                    end
                end
                
                STATE_RECEIVING_FORK: begin
                    // Simplified fork receiving - just update last_update
                    last_update <= incoming_var;
                    clause_index <= 0;
                    sat_flag <= 1'b1;
                end
                
                STATE_ABORTING: begin
                    outgoing_var <= last_update;
                    outgoing_var_valid <= 1'b1;
                    outgoing_msg_type <= MSG_SUBSTITUTION_MASK;
                    node_busy <= 4'b1111;
                end
            endcase
            
            // SAT detection
            if (current_state == STATE_PROCESSING && clause_index == NUM_CLAUSES - 1 && sat_flag) begin
                sat_found <= 1'b1;
            end
        end
    end

endmodule 