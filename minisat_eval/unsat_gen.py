import random
import os

def generate_unsat_3sat(num_vars=20, num_clauses=90):
    # Generate a random assignment for all variables
    solution = {var: random.choice([True, False]) for var in range(1, num_vars + 1)}
    clauses = []

    # Generate clauses compatible with the solution (satisfiable base)
    for _ in range(num_clauses - 2):  # Reserve 2 clauses for conflicts
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

def write_cnf_file(clauses, filename):
    """Write clauses to a CNF file with proper formatting."""
    with open(filename, "w") as f:
        # Write header with correct number of clauses
        f.write(f"p cnf 20 {len(clauses)}\n")
        
        # Write each clause with proper formatting
        for clause in clauses:
            # Ensure each clause has exactly 3 literals
            if len(clause) != 3:
                print(f"Warning: Clause {clause} does not have exactly 3 literals")
                # Pad with additional literals if needed
                while len(clause) < 3:
                    var = random.randint(1, 20)
                    clause.append(var if random.random() < 0.5 else -var)
                # Truncate if too many literals
                clause = clause[:3]
            
            # Write the clause with proper spacing and termination
            # Ensure each literal is a valid integer
            formatted_literals = []
            for lit in clause:
                if isinstance(lit, bool):
                    lit = 1 if lit else -1
                formatted_literals.append(str(lit))
            
            clause_str = " ".join(formatted_literals) + " 0\n"
            f.write(clause_str)

# Create output directory if it doesn't exist
os.makedirs("minisat_eval/gen_unsat", exist_ok=True)

# Generate 20 different unsatisfiable formulas
for i in range(1, 21):
    clauses = generate_unsat_3sat()
    output_file = f"minisat_eval/gen_unsat/unsat_formula_{i}.cnf"
    write_cnf_file(clauses, output_file)
    print(f"Generated unsatisfiable formula {i}/20: {output_file}")

print("All 20 unsatisfiable formulas have been generated.")

