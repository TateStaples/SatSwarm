# SatSwarm Implementation
## Core
The core is the main commonent of our distributed computing architecture. Each core should be able to complete the entire DPLL algorithm independently but will outsource work where possible.
### Substitutions
1. Select a variable to substitue, to start we will always choose the first unassigned variable
2. Choose one assignment to make.if neighbors are available, fork the current state of the CNF assignment buffer to a neighbor, else mark the assignment as speculative
3. Query the Look Up Table (LUT) for a bitmask of where that variable can be found
4. Loop through the CNF assignment buffer and apply the substitution to each clause (steps 5+)
5. If the clause has no symbolic variables, skip it
6. use the bitmask to determine if the variable is +variable, -variable, or other variable and apply the substitution
6.5. If the clause is now a unit clause, add it to the unit clause buffer
7. If the clause is now a contradiction, initiate backtracking (UNSAT)
8. If the clause is now a tautology (all true), mark it as not symbolic
9. If all clauses are not symbolic, broadcast success (SAT). Otherwise, continue to the next variable (step 1)

### CNF Assignment Buffer
To start we are designing a 3-SAT solver so each clause will be represented by 3 bits. 0 = symbolic, 1 = false. Trues are not necessary as they will make the whole clause true. 

### Backtracking & Speculative Assignments
When making a speculative assignment that cannot be forked, the core will push the id of the variable and the assignment to a stack (assignment may not be necessary if we always choose 0 first). When backtracking, the core will pop the stack and undo the assignment. Next, it will query the LUT for the variable id and stream back clauses that need to be reset. The core will then loop through the CNF assignment buffer and reset the clauses that were streamed back. Finally, the core will set the variable to the opposite assignment as non-speculative and continue the substitution process.

### Forking
The core will fork work to a neighbor if it is available. The neighbor will then take the current state of the CNF assignment buffer and continue the substitution process. If the neighbor finds a contradiction, it will send a message back to the original core to initiate backtracking. If the neighbor finds a solution, it will send a message back to the original core to broadcast success.

## Messaging
When forking work over to a neighbor, the sender must communicate which variables have already been assigned and the current state of the CNF assignment buffer.
## LUT Table
A static global lookup table with a buffer of u8 unique ids for each variable

### Substitution
When a core wants to assign a value into a given variable, it will query its LUT for the variable id and the LUT will stream back a bitmask for each clause representing whether each term in the clause is +variable, -variable, or other variable. This will then be used by the core for its substitution

### Reset
When a node backtracks to a previous speculative assingment, it must undo all of the substitutions that came after this assignment. The LUT supports querying at a given variable and then stream clauses back with bitmasks of what should be considered symbolic and what should remain.
# Extensions
- Unit propagation
- Investigate alternative network topologies