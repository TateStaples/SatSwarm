module node (
    input wire clk,
    input wire rst_n,
    
    // Input interface
    input wire [7:0] incoming_var,
    input wire incoming_var_valid,
    input wire [1:0] incoming_msg_type,  // 00: None, 01: Fork, 10: SubstitutionMask
    input wire [2:0] incoming_mask,
    
    // Output interface
    output reg [7:0] outgoing_var,
    output reg outgoing_var_valid,
    output reg [1:0] outgoing_msg_type,
    output reg [2:0] outgoing_mask,
    output reg node_busy,
    output reg sat_found
);

    // Message type encoding
    localparam MSG_NONE = 2'b00;
    localparam MSG_FORK = 2'b01;
    localparam MSG_SUBSTITUTION_MASK = 2'b10;
    
    // Internal registers
    reg [7:0] var_counter;
    reg [3:0] clause_counter;
    reg processing;
    
    // Simplified state machine with minimal logic
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            var_counter <= 8'd0;
            clause_counter <= 4'd0;
            outgoing_var <= 8'd0;
            outgoing_var_valid <= 1'b0;
            outgoing_msg_type <= MSG_NONE;
            outgoing_mask <= 3'd0;
            node_busy <= 1'b0;
            sat_found <= 1'b0;
            processing <= 1'b0;
        end else begin
            // Default values to reduce logic
            outgoing_var_valid <= 1'b0;
            outgoing_msg_type <= MSG_NONE;
            
            if (!processing) begin
                // IDLE state
                if (incoming_var_valid && incoming_msg_type == MSG_FORK) begin
                    processing <= 1'b1;
                    var_counter <= incoming_var;
                    node_busy <= 1'b1;
                end
            end else begin
                // PROCESSING state
                if (incoming_msg_type == MSG_SUBSTITUTION_MASK) begin
                    outgoing_var <= var_counter;
                    outgoing_var_valid <= 1'b1;
                    outgoing_msg_type <= MSG_SUBSTITUTION_MASK;
                    outgoing_mask <= incoming_mask;
                    
                    if (clause_counter == 4'd15) begin  // Process 16 clauses
                        // Fork logic
                        outgoing_var <= var_counter + 1'b1;
                        outgoing_var_valid <= 1'b1;
                        outgoing_msg_type <= MSG_FORK;
                        processing <= 1'b0;
                        node_busy <= 1'b0;
                        
                        if (var_counter == 8'd255) begin  // Max variable reached
                            sat_found <= 1'b1;
                        end
                        
                        clause_counter <= 4'd0;
                    end else begin
                        clause_counter <= clause_counter + 1'b1;
                    end
                end
            end
        end
    end

endmodule 