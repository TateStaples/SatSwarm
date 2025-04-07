import subprocess
import csv
import re
import os
import glob

def run_minisat(formula_file):
    """Run MiniSat on a formula file and return the output."""
    try:
        result = subprocess.run(['minisat', formula_file], 
                               capture_output=True, 
                               text=True, 
                               check=True)
        return result.stdout
    except subprocess.CalledProcessError as e:
        # MiniSat returns non-zero exit code for UNSAT (20) or parsing errors (10)
        if e.returncode == 20:  # UNSAT
            return e.stdout
        elif e.returncode == 10:  # Parsing error
            print(f"Parsing error in {formula_file}. Error message: {e.stderr}")
            # Try to read the file to check for issues
            try:
                with open(formula_file, 'r') as f:
                    content = f.read()
                    print(f"File content (first 500 chars): {content[:500]}")
            except Exception as read_err:
                print(f"Could not read file: {read_err}")
            return None
        else:
            print(f"Error running MiniSat on {formula_file}: {e}")
            return None

def extract_minisat_info(output):
    """Extract relevant information from MiniSat output."""
    if not output:
        return None
    
    info = {}
    
    # Extract problem statistics
    num_vars_match = re.search(r'Number of variables:\s+(\d+)', output)
    num_clauses_match = re.search(r'Number of clauses:\s+(\d+)', output)
    parse_time_match = re.search(r'Parse time:\s+([\d.]+) s', output)
    simplification_time_match = re.search(r'Simplification time:\s+([\d.]+) s', output)
    
    if num_vars_match:
        info['num_vars'] = int(num_vars_match.group(1))
    if num_clauses_match:
        info['num_clauses'] = int(num_clauses_match.group(1))
    if parse_time_match:
        info['parse_time'] = float(parse_time_match.group(1))
    if simplification_time_match:
        info['simplification_time'] = float(simplification_time_match.group(1))
    
    # Extract search statistics
    conflicts_match = re.search(r'conflicts\s+:\s+(\d+)', output)
    decisions_match = re.search(r'decisions\s+:\s+(\d+)', output)
    propagations_match = re.search(r'propagations\s+:\s+(\d+)', output)
    conflict_literals_match = re.search(r'conflict literals\s+:\s+(\d+)', output)
    memory_used_match = re.search(r'Memory used\s+:\s+([\d.]+) MB', output)
    cpu_time_match = re.search(r'CPU time\s+:\s+([\d.]+) s', output)
    
    if conflicts_match:
        info['conflicts'] = int(conflicts_match.group(1))
    if decisions_match:
        info['decisions'] = int(decisions_match.group(1))
    if propagations_match:
        info['propagations'] = int(propagations_match.group(1))
    if conflict_literals_match:
        info['conflict_literals'] = int(conflict_literals_match.group(1))
    if memory_used_match:
        info['memory_used_mb'] = float(memory_used_match.group(1))
    if cpu_time_match:
        info['cpu_time'] = float(cpu_time_match.group(1))
    
    # Determine if formula is SAT or UNSAT
    if 'UNSATISFIABLE' in output:
        info['result'] = 'UNSAT'
    elif 'SATISFIABLE' in output:
        info['result'] = 'SAT'
    else:
        info['result'] = 'UNKNOWN'
    
    return info

def check_cnf_file(file_path):
    """Check if a CNF file is valid."""
    try:
        with open(file_path, 'r') as f:
            lines = f.readlines()
            
        # Check header line
        if not lines or not lines[0].startswith('p cnf'):
            print(f"Invalid header in {file_path}")
            return False
            
        # Parse header
        header_parts = lines[0].strip().split()
        if len(header_parts) != 4:
            print(f"Invalid header format in {file_path}: {lines[0]}")
            return False
            
        num_vars = int(header_parts[2])
        num_clauses = int(header_parts[3])
        
        # Count clauses
        clause_count = 0
        for line in lines[1:]:
            if line.strip() and not line.startswith('c'):
                clause_count += 1
                
        if clause_count != num_clauses:
            print(f"Clause count mismatch in {file_path}: expected {num_clauses}, found {clause_count}")
            return False
            
        return True
    except Exception as e:
        print(f"Error checking CNF file {file_path}: {e}")
        return False

def main():
    # Create output directory if it doesn't exist
    os.makedirs("unsat_generator/results", exist_ok=True)
    
    # Get all formula files
    formula_files = glob.glob("unsat_generator/gen_unsat/unsat_formula_*.cnf")
    
    if not formula_files:
        print("No formula files found. Please run unsat_gen.py first.")
        return
    
    # Prepare CSV file
    csv_file = "unsat_generator/results/minisat_results.csv"
    fieldnames = [
        'formula_file', 'num_vars', 'num_clauses', 'parse_time', 'simplification_time',
        'conflicts', 'decisions', 'propagations', 'conflict_literals', 
        'memory_used_mb', 'cpu_time', 'result'
    ]
    
    with open(csv_file, 'w', newline='') as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        
        # Process each formula
        for formula_file in sorted(formula_files):
            print(f"Processing {formula_file}...")
            
            # First check if the CNF file is valid
            if not check_cnf_file(formula_file):
                print(f"  Skipping invalid CNF file: {formula_file}")
                continue
                
            output = run_minisat(formula_file)
            info = extract_minisat_info(output)
            
            if info:
                info['formula_file'] = os.path.basename(formula_file)
                writer.writerow(info)
                print(f"  Result: {info['result']}, CPU time: {info.get('cpu_time', 'N/A')}s")
            else:
                print(f"  Failed to extract information from {formula_file}")
    
    print(f"Results written to {csv_file}")

if __name__ == "__main__":
    main()
