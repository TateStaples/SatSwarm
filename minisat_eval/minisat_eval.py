#!/usr/bin/env python3

import os
import subprocess
import csv
import re
from pathlib import Path

def parse_minisat_output(output):
    """Parse MiniSat output to extract result and timing information."""
    # Check if the formula is satisfiable
    is_sat = "SATISFIABLE" in output and "UNSATISFIABLE" not in output
    
    # Extract CPU time using regex
    cpu_time_match = re.search(r'CPU time\s+: ([\d.]+) s', output)
    cpu_time = float(cpu_time_match.group(1)) if cpu_time_match else 0.0
    
    # Extract problem statistics
    vars_match = re.search(r'Number of variables:\s+(\d+)', output)
    clauses_match = re.search(r'Number of clauses:\s+(\d+)', output)
    
    num_vars = int(vars_match.group(1)) if vars_match else 0
    num_clauses = int(clauses_match.group(1)) if clauses_match else 0
    
    # Extract search statistics
    conflicts_match = re.search(r'conflicts\s+: (\d+)', output)
    decisions_match = re.search(r'decisions\s+: (\d+)', output)
    propagations_match = re.search(r'propagations\s+: (\d+)', output)
    
    conflicts = int(conflicts_match.group(1)) if conflicts_match else 0
    decisions = int(decisions_match.group(1)) if decisions_match else 0
    propagations = int(propagations_match.group(1)) if propagations_match else 0
    
    return {
        'is_sat': is_sat,
        'cpu_time': cpu_time,
        'num_vars': num_vars,
        'num_clauses': num_clauses,
        'conflicts': conflicts,
        'decisions': decisions,
        'propagations': propagations
    }

def run_minisat(cnf_file):
    """Run MiniSat on a CNF file and return the result and timing information."""
    # Run MiniSat with verbose output
    result = subprocess.run(['minisat', cnf_file], 
                          capture_output=True, 
                          text=True)
    
    # Parse the output
    output = result.stdout
    stderr = result.stderr
    
    # Parse MiniSat output
    parsed = parse_minisat_output(output)
    
    print(f"\nProcessing {cnf_file}:")
    print("MiniSat output:")
    print(output)
    if stderr:
        print("MiniSat stderr:")
        print(stderr)
    print(f"Result: {'SAT' if parsed['is_sat'] else 'UNSAT'}")
    print(f"CPU Time: {parsed['cpu_time']:.4f} seconds")
    print(f"Variables: {parsed['num_vars']}, Clauses: {parsed['num_clauses']}")
    print(f"Conflicts: {parsed['conflicts']}, Decisions: {parsed['decisions']}, Propagations: {parsed['propagations']}")
    print("-" * 80)
    
    return {
        'filename': os.path.basename(cnf_file),
        'result': 'SAT' if parsed['is_sat'] else 'UNSAT',
        'time': parsed['cpu_time'],
        'num_vars': parsed['num_vars'],
        'num_clauses': parsed['num_clauses'],
        'conflicts': parsed['conflicts'],
        'decisions': parsed['decisions'],
        'propagations': parsed['propagations']
    }

def process_directory(directory):
    """Process all CNF files in a directory."""
    results = []
    print(f"\nProcessing directory: {directory}")
    for file in os.listdir(directory):
        if file.endswith('.cnf'):
            full_path = os.path.join(directory, file)
            result = run_minisat(full_path)
            results.append(result)
    return results

def main():
    # Create output directory if it doesn't exist
    os.makedirs('logs_minisat', exist_ok=True)
    
    # Process both sat and unsat directories
    all_results = []
    
    # Process SAT directory
    sat_dir = 'tests/random/sat'
    if os.path.exists(sat_dir):
        sat_results = process_directory(sat_dir)
        all_results.extend(sat_results)
    else:
        print(f"Warning: SAT directory not found at {sat_dir}")
    
    # Process UNSAT directory
    unsat_dir = 'tests/random/unsat'
    if os.path.exists(unsat_dir):
        unsat_results = process_directory(unsat_dir)
        all_results.extend(unsat_results)
    else:
        print(f"Warning: UNSAT directory not found at {unsat_dir}")
    
    # Write results to CSV
    output_file = 'logs_minisat/minisat_results.csv'
    with open(output_file, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=[
            'filename', 'result', 'time', 'num_vars', 'num_clauses',
            'conflicts', 'decisions', 'propagations'
        ])
        writer.writeheader()
        writer.writerows(all_results)
    
    print(f"\nResults written to {output_file}")
    print(f"Total files processed: {len(all_results)}")

if __name__ == '__main__':
    main() 