# Extensions
This is me writing out my thoughts on some small performance enhancements above and beyond our initial backtracking algorithm

## Removing the ClauseTable
Would it make sense to remove the claues table. We added it in as a space optimization but it seems like the synchrnoization with a static object like this is causing significant problems. What could be solved if we remove this?
- Unit Propagation would become significantly easier
- Pipelining would become significantly easier as you don't need a higher number of message ports

## Unit Propagation
This is part of the full DPLL algorithm is seems to be a vital optimization. Essentially it automatically sets all single (symbolic) variable clauses to true because that is the only way for the whole item to be sat. This drastically reduces the amount of forking in the operation, especially in 3SAT.

For implementation this isn't trivial because in each of our nodes we are lacking Variable IDs and additionally we initially are assuming completely in order variable setting. Relaxing this assumption will cause issues for rolling back speculative state.
I'm proposings an updates array of length Vars, where 0=not set, 1=root set, 2=second layer set, ... up to a max of VarID. You would request updates on a specific level. The ClauseTable would send resets for all later udpates and unit propagations would get added in with the level that implies them. (TODO: how do they figure out their

## Sparse Networking Optimzatin
The updates from our LUT tend to be very sparse (most terms aren't in most clauses), so there are potential compute and network improvements from using an update format more specific to this structure (ie [skips, update]). Leveraging this would probably require the ClauseTable to also loop over the data in a more time efficient manner

## Pipelining
This is another slightly less trivial optimization that I think would be worthwhile. Right now we are assuming looping over all clauses on each assignment. This is a long delay between network effects and it seems like their is room for speculative assumptions of no unsat.

This requires each node to be able to recieve more than one message per clock cycle. This could also be 

## Clause Learning
This is probably not going to happen because the implementation sounds complicated, propagation of learned clauses sounds moderately difficult, and such.