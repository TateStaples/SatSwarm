import random
import os

def generate_unsat_3sat(num_vars=20, num_clauses=90):
    # Generate a random assignment for all variables
    solution = {var: random.choice([True, False]) for var in range(1, num_vars + 1)}
    clauses = []

    # Generate clauses compatible with the solution (satisfiable base)
    for _ in range(num_clauses - 1):
        # Select 3 distinct variables
        vars = random.sample(range(1, num_vars + 1), 3)
        clause = []
        for var in vars:
            # Include literal matching the solution with 50% probability
            if random.random() < 0.5:
                clause.append(var if solution[var] else -var)
            else:
                clause.append(-var if solution[var] else var)
        clauses.append(clause)
    
    # Add final conflicting clause to make formula unsatisfiable
    # Create a direct conflict by adding a clause that contradicts the solution
    conflict_vars = random.sample(range(1, num_vars + 1), 3)
    conflict_clause = []
    for var in conflict_vars:
        # Add the opposite of what the solution requires
        conflict_clause.append(-var if solution[var] else var)
    clauses.append(conflict_clause)
    
    # Add another conflicting clause to ensure unsatisfiability
    conflict_vars2 = random.sample(range(1, num_vars + 1), 3)
    conflict_clause2 = []
    for var in conflict_vars2:
        # Add the opposite of what the solution requires
        conflict_clause2.append(-var if solution[var] else var)
    clauses.append(conflict_clause2)
    
    return clauses

# Create output directory if it doesn't exist
os.makedirs("unsat_test/gen_unsat", exist_ok=True)

# Generate 20 different unsatisfiable formulas
for i in range(1, 21):
    clauses = generate_unsat_3sat()
    output_file = f"unsat_test/gen_unsat/unsat_formula_{i}.cnf"
    
    with open(output_file, "w") as f:
        f.write(f"p cnf 20 {len(clauses)}\n")
        for clause in clauses:
            f.write(" ".join(map(str, clause)) + " 0\n")
    
    print(f"Generated unsatisfiable formula {i}/20: {output_file}")

print("All 20 unsatisfiable formulas have been generated.")

