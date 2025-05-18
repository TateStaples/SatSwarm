import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Define the data files
data_dir = 'data'
# torus_file = os.path.join(data_dir, 'eval_set-Torus(8, 8)-64-1-1000000.csv')
torus_file = os.path.join(data_dir, 'satlib-Torus(8, 8)-64-100-50.csv')
# dense_file = os.path.join(data_dir, 'eval_set-Dense(64)-64-1-1000000.csv')
dense_file = os.path.join(data_dir, 'satlib-Dense(64)-64-100-50.csv')

# Clock frequency in Hz (1 GHz = 1,000,000,000 Hz)
clock_frequency = 1_000_000_000  # 1 GHz

# Read the data
torus_data = pd.read_csv(torus_file)
dense_data = pd.read_csv(dense_file)

# Extract test names
def extract_test_name(path):
    return path.split('/')[-1]

torus_data['Test'] = torus_data['Test Path'].apply(extract_test_name)
dense_data['Test'] = dense_data['Test Path'].apply(extract_test_name)

# Calculate Minisat time in seconds (convert from nanoseconds)
torus_data['Minisat Time (s)'] = torus_data['Minisat Speed (ns)'] / 1_000_000_000
dense_data['Minisat Time (s)'] = dense_data['Minisat Speed (ns)'] / 1_000_000_000

# Calculate simulation time in seconds (cycles / frequency)
torus_data['Simulation Time (s)'] = torus_data['Simulated Cycles'] / clock_frequency
dense_data['Simulation Time (s)'] = dense_data['Simulated Cycles'] / clock_frequency

# Create a combined dataframe for plotting
combined_data = pd.DataFrame({
    'Test': torus_data['Test'],
    'Minisat Time (s)': torus_data['Minisat Time (s)'],
    'Torus(8, 8) Time (s)': torus_data['Simulation Time (s)'],
    'Dense(64) Time (s)': dense_data['Simulation Time (s)']
})

# Sort by test name for consistent ordering
combined_data = combined_data.sort_values('Test')

# Set up the plot
plt.figure(figsize=(12, 8))
bar_width = 0.25
index = np.arange(len(combined_data))

# Create bars for each method
plt.bar(index, combined_data['Minisat Time (s)'], bar_width, label='Minisat', color='blue')
plt.bar(index + bar_width, combined_data['Torus(8, 8) Time (s)'], bar_width, label='Torus(8, 8)', color='skyblue')
plt.bar(index + 2*bar_width, combined_data['Dense(64) Time (s)'], bar_width, label='Dense(64)', color='salmon')

# Add labels and title
plt.xlabel('Test Case')
plt.ylabel('Execution Time (seconds)')
plt.title('Comparison of Execution Time: Minisat vs. Torus(8, 8) vs. Dense(64)')
plt.xticks(index + bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the plot
plt.savefig('minisat_comparison.png', dpi=300)
print("Plot saved as 'minisat_comparison.png'")

# Also create a log-scale version for better visualization
plt.figure(figsize=(12, 8))
plt.bar(index, combined_data['Minisat Time (s)'], bar_width, label='Minisat', color='blue')
plt.bar(index + bar_width, combined_data['Torus(8, 8) Time (s)'], bar_width, label='Torus(8, 8)', color='skyblue')
plt.bar(index + 2*bar_width, combined_data['Dense(64) Time (s)'], bar_width, label='Dense(64)', color='salmon')

plt.xlabel('Test Case')
plt.ylabel('Execution Time (seconds, log scale)')
plt.title('Comparison of Execution Time: Minisat vs. Torus(8, 8) vs. Dense(64) (Log Scale)')
plt.xticks(index + bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')
plt.tight_layout()

# Save the log-scale plot
plt.savefig('minisat_comparison_log.png', dpi=300)
print("Log-scale plot saved as 'minisat_comparison_log.png'")

# Calculate speedup factors
combined_data['Torus Speedup'] = combined_data['Minisat Time (s)'] / combined_data['Torus(8, 8) Time (s)']
combined_data['Dense Speedup'] = combined_data['Minisat Time (s)'] / combined_data['Dense(64) Time (s)']

# Create a bar plot for speedup factors
plt.figure(figsize=(12, 8))
plt.bar(index, combined_data['Torus Speedup'], bar_width, label='Torus(8, 8)', color='skyblue')
plt.bar(index + bar_width, combined_data['Dense Speedup'], bar_width, label='Dense(64)', color='salmon')

plt.xlabel('Test Case')
plt.ylabel('Speedup Factor (Minisat Time / Simulation Time)')
plt.title('Speedup Factor: How Much Faster Minisat is Compared to Simulation')
plt.xticks(index + bar_width/2, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the speedup plot
plt.savefig('minisat_speedup.png', dpi=300)
print("Speedup plot saved as 'minisat_speedup.png'")

# Print summary statistics
print("\nSummary Statistics:")
print(f"{'Method':<15} {'Mean Time (s)':<15} {'Median Time (s)':<15} {'Min Time (s)':<15} {'Max Time (s)':<15}")
print("-" * 75)
for method in ['Minisat Time (s)', 'Torus(8, 8) Time (s)', 'Dense(64) Time (s)']:
    time_data = combined_data[method]
    print(f"{method:<15} {time_data.mean():<15.6f} {time_data.median():<15.6f} {time_data.min():<15.6f} {time_data.max():<15.6f}")

# Print speedup statistics
print("\nSpeedup Statistics:")
print(f"{'Topology':<15} {'Mean Speedup':<15} {'Median Speedup':<15} {'Min Speedup':<15} {'Max Speedup':<15}")
print("-" * 75)
for topology in ['Torus Speedup', 'Dense Speedup']:
    speedup_data = combined_data[topology]
    print(f"{topology:<15} {speedup_data.mean():<15.2f} {speedup_data.median():<15.2f} {speedup_data.min():<15.2f} {speedup_data.max():<15.2f}")

# Calculate correlation coefficients
torus_corr = combined_data['Minisat Time (s)'].corr(combined_data['Torus(8, 8) Time (s)'])
dense_corr = combined_data['Minisat Time (s)'].corr(combined_data['Dense(64) Time (s)'])

print("\nCorrelation with Minisat Time:")
print(f"Torus(8, 8): {torus_corr:.4f}")
print(f"Dense(64): {dense_corr:.4f}")

# Create boxplot comparison
plt.figure(figsize=(10, 6))
boxplot_data = [
    combined_data['Minisat Time (s)'],
    combined_data['Torus(8, 8) Time (s)'],
    combined_data['Dense(64) Time (s)']
]
plt.boxplot(boxplot_data, labels=['Minisat', 'Torus(8, 8)', 'Dense(64)'])
plt.ylabel('Execution Time (seconds)')
plt.title('Distribution of Execution Times')
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')  # Use log scale for better visualization
plt.tight_layout()

# Save the boxplot
plt.savefig('execution_time_boxplot.png', dpi=300)
print("Boxplot saved as 'execution_time_boxplot.png'") 