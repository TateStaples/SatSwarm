# SAT Solver Hardware Accelerator
> Tate and Shaan

The goal of the project is to make a hardware accelerator for logical verification problems. This can be broken down into several subproblems for incremental progress. 
## Basic Project: SAT Solver (DPLL)
A SAT solver is an algorithm to check if a boolean formula (for example A & B & !C) has any assignment of variables (ie ABC) that make the formula true. The formula is canonically inputted in the conjunctive normal form [ie (A&B) OR (B&D) OR (A&!C) OR …].

Davis–Putnam–Logemann–Loveland (DPLL) algorithm is a standard method of solving this essential problem: make a decision, simplify, and backtrack to the last decision if wrong. The idea for the project is that the logic of the algorithm is fairly simple and embarrassingly parallel (fork on assignment).


### Ideas for Hardware Implementation
The idea for hardware implementation is to build a tree of processors with a work-stealing protocol. When assigning a variable, if the processor has any free neighbors, it will choose one assignment for itself and the dual for the neighbor. If the child ever returns SAT, broadcast and return. If the child returns UNSAT, note the explored branch and free the child for further work.

We intend on representing conjunctive normal form (CNF) with a buffer of OR’ed terms that are either symbolic or FALSE (True literals would make the term immediately cancel out). We are still working on how to efficiently implement assignment and unit propagation. Additionally, we still need to figure out how to store the parent routing of where to propagate UNSAT if the parent starts helping a child with its work. 
### Easy & Fun Application
The article that first got me interested in the topic was talking about highly optimized Soduko solvers. It would be an easily communicable poster/presentation if we demonstrated and/or tested our project on this challenge.
## Implementation
### Framework
To model this we will be building a cycle-accurate simulator in Rust. We will have a large grid of nodes that we will parameterize, and then later evaluate (i.e. for grid size, communication latencies, hardware latencies, buffer sizes and so on). There will be lots of custom interconnect/networking protocols to be thought through and built here.
### Realism
One of the things we spoke about while thinking about this idea was its realism. For example, we will need to figure out whether it even is realistic to send data between nodes in one cycle, and if so how fast can this clock frequency be? This is especially important for correct evaluation of results (next section). One of our ideas (depending on feasible this is, possibly an extension) is to make an estimate node in Verilog and use an open source toolchain of Yosys + Google Skywater PDK => OpenSTA => timing analysis to figure out a minimum clock frequency (with some additional safety margin).
## Testing and Evaluation
### Functional
We will need to check whether our accelerator is functionally equivalent to SAT solvers. Do we even get the correct answer? For this we will use the SATLIB Benchmarks and make sure computation results are equivalent for our accelerator and a well known SAT solver, miniSAT. 
### Performance
Similarly, we will need to evaluate whether our accelerator is even beneficial. Right now we are thinking of running our simulator, and seeing how many cycles it takes to solve a problem and then using our realistic clock frequency to estimate a time to compute. Then, we intend to run a SAT solver binary (such as miniSAT) on a single-threaded instance of gem5 or similar processor simulator. We will compare the time taken by us to solve a SAT problem, to a solution done by traditional CPUs to hopefully show the performance benefit of our design.
## Extensions
### Model Resolution
The most basic implementation of the computer will only return SAT/UNSAT. It is much more useful if it returns the assignment in the SAT case and sufficient syndrome in the UNSAT case to make a checkable proof.
### Conflict-Driven Clause Learning (CDCL)
One common extension for the DPLL algorithm is conflict assignment. Instead of backtracking one step on UNSAT, you figure out which assignment was responsible by maintaining traces that represent the underlying dependency graph. Then you jump back to the most recent breaking decision and do Clause Learning by adding a new Lemma representing a higher-level assignment that you know will lead to UNSAT.

There seems to be work on clause learning in hardware dating back to 2007 so there should be some guiding literature on implementation if you choose to go this route.
Satisfiability Modulo Theories (SMT)
The next extension that would be interesting would be to make it more applicable to real-world software verification by combining it with more theories (ie math axioms, array rules, strings, algebraic data types) that would allow it to do full/limited software verification.

