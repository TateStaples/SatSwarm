

## SAT-Accel

https://dl.acm.org/doi/10.1145/3706628.3708869

idea is that algorithmic advances in SAT solvers are much more important than hardware advances (ie 2000s algorithm on modern hardware is slower than 2000s hardware with modern algorithms).

"This limitation is why there are very few parallel SAT
solvers on CPUs and GPUs. In particular, GPUs are based on the
Single Instruction Multiple Data (SIMD) architecture. SAT solving,
however, requires complex control flow (Algorithm 2), something
that a FPGA is flexible enough to support"

- number of variables and clauses to solve is something that needs to be looked at
"SAT-Accel supports 32k variables and 131k clauses, an 8x
increase in the number of clauses compared to other acceler-
ated stand-alone solvers."


Main algorithmic improvements over time, maybe it is wise to incorporate these for even more speedup:
```
•Conflict Driven Clause Learning (CDCL) 
    - backing up more than one level and continuing work
    - we have this
•Clause Minimization 
    - remove redundant clauses
    - do not have this
•Clause Deletion
    - "Clauses with the least number of unique decision levels are believed to have higher quality since they are more likely to lead to propagation."
    - "Modern solvers delete clauses of low quality after a certain amount of learning or during a restart."
    - hard to do because we seemingly need global information?
    - do not have this
•Solver Restart Policy
    - Restart from the top of the tree every now and then to avoid getting stuck
    - Kind of irrelevant for us since we are not single threaded and do breath
•Phase Saving
    - again to do with saving state when backtracking, irrelevant since single thread based idea
•Variable Decision Heuristic
    - Heuristic to decide which variable to branch on
    - could be interesting, would have to adapt to a system without global information
•Efficient Clause Checking (ECC)
    - This reduces the bloat due to lack of spatial and temporal locality when assigning variables
    - 2 Watched Literal scheme (look into more)
    - This could be really interesting and useful.
```

Propagate (check where other vars are assigned) is very parallelizable

