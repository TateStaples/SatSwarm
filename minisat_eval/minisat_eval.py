#!/usr/bin/env python3

import os
import subprocess
import csv
import time
from pathlib import Path

def run_minisat(cnf_file):
    """Run MiniSat on a CNF file and return the result and timing information."""
    start_time = time.time()
    
    # Run MiniSat with verbose output
    result = subprocess.run(['minisat', cnf_file], 
                          capture_output=True, 
                          text=True)
    
    end_time = time.time()
    duration = end_time - start_time
    
    # Parse the output
    output = result.stdout
    stderr = result.stderr
    
    # MiniSat outputs SATISFIABLE/UNSATISFIABLE to stdout
    is_sat = "SATISFIABLE" in output and "UNSATISFIABLE" not in output
    
    print(f"\nProcessing {cnf_file}:")
    print("MiniSat output:")
    print(output)
    if stderr:
        print("MiniSat stderr:")
        print(stderr)
    print(f"Result: {'SAT' if is_sat else 'UNSAT'}")
    print(f"Time: {duration:.4f} seconds")
    print("-" * 80)
    
    return {
        'filename': os.path.basename(cnf_file),
        'result': 'SAT' if is_sat else 'UNSAT',
        'time': duration
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
        writer = csv.DictWriter(f, fieldnames=['filename', 'result', 'time'])
        writer.writeheader()
        writer.writerows(all_results)
    
    print(f"\nResults written to {output_file}")
    print(f"Total files processed: {len(all_results)}")

if __name__ == '__main__':
    main() 