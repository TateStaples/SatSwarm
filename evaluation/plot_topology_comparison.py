import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Define the data files
data_dir = 'data'
torus_file = os.path.join(data_dir, 'eval_set-Torus(8, 8)-64-1-1000000.csv')
grid_file = os.path.join(data_dir, 'eval_set-Grid(8, 8)-64-1-1000000.csv')
dense_file = os.path.join(data_dir, 'eval_set-Dense(64)-64-1-1000000.csv')

# Read the data
torus_data = pd.read_csv(torus_file)
grid_data = pd.read_csv(grid_file)
dense_data = pd.read_csv(dense_file)

# Extract test names and simulation cycles
def extract_test_name(path):
    return path.split('/')[-1]

torus_data['Test'] = torus_data['Test Path'].apply(extract_test_name)
grid_data['Test'] = grid_data['Test Path'].apply(extract_test_name)
dense_data['Test'] = dense_data['Test Path'].apply(extract_test_name)

# Create a combined dataframe for plotting
combined_data = pd.DataFrame({
    'Test': torus_data['Test'],
    'Torus(8, 8)': torus_data['Simulated Cycles'],
    'Grid(8, 8)': grid_data['Simulated Cycles'],
    'Dense(64)': dense_data['Simulated Cycles']
})

# Sort by test name for consistent ordering
combined_data = combined_data.sort_values('Test')

# Set up the plot
plt.figure(figsize=(12, 8))
bar_width = 0.25
index = np.arange(len(combined_data))

# Create bars for each topology
plt.bar(index, combined_data['Torus(8, 8)'], bar_width, label='Torus(8, 8)', color='skyblue')
plt.bar(index + bar_width, combined_data['Grid(8, 8)'], bar_width, label='Grid(8, 8)', color='lightgreen')
plt.bar(index + 2*bar_width, combined_data['Dense(64)'], bar_width, label='Dense(64)', color='salmon')

# Add labels and title
plt.xlabel('Test Case')
plt.ylabel('Simulation Cycles')
plt.title('Comparison of Simulation Cycles Across Topologies')
plt.xticks(index + bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the plot
plt.savefig('topology_comparison.png', dpi=300)
print("Plot saved as 'topology_comparison.png'")

# Also create a log-scale version for better visualization
plt.figure(figsize=(12, 8))
plt.bar(index, combined_data['Torus(8, 8)'], bar_width, label='Torus(8, 8)', color='skyblue')
plt.bar(index + bar_width, combined_data['Grid(8, 8)'], bar_width, label='Grid(8, 8)', color='lightgreen')
plt.bar(index + 2*bar_width, combined_data['Dense(64)'], bar_width, label='Dense(64)', color='salmon')

plt.xlabel('Test Case')
plt.ylabel('Simulation Cycles (log scale)')
plt.title('Comparison of Simulation Cycles Across Topologies (Log Scale)')
plt.xticks(index + bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')
plt.tight_layout()

# Save the log-scale plot
plt.savefig('topology_comparison_log.png', dpi=300)
print("Log-scale plot saved as 'topology_comparison_log.png'")

# Create a box plot to show the distribution of cycles
plt.figure(figsize=(10, 6))
box_data = [
    combined_data['Torus(8, 8)'],
    combined_data['Grid(8, 8)'],
    combined_data['Dense(64)']
]
plt.boxplot(box_data, labels=['Torus(8, 8)', 'Grid(8, 8)', 'Dense(64)'])
plt.ylabel('Simulation Cycles')
plt.title('Distribution of Simulation Cycles Across Topologies')
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the box plot
plt.savefig('topology_boxplot.png', dpi=300)
print("Box plot saved as 'topology_boxplot.png'")

# Create a box plot with log scale
plt.figure(figsize=(10, 6))
plt.boxplot(box_data, labels=['Torus(8, 8)', 'Grid(8, 8)', 'Dense(64)'])
plt.ylabel('Simulation Cycles (log scale)')
plt.title('Distribution of Simulation Cycles Across Topologies (Log Scale)')
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')
plt.tight_layout()

# Save the log-scale box plot
plt.savefig('topology_boxplot_log.png', dpi=300)
print("Log-scale box plot saved as 'topology_boxplot_log.png'")

# Print summary statistics
print("\nSummary Statistics:")
print(f"{'Topology':<15} {'Mean Cycles':<15} {'Median Cycles':<15} {'Min Cycles':<15} {'Max Cycles':<15}")
print("-" * 75)
for topology in ['Torus(8, 8)', 'Grid(8, 8)', 'Dense(64)']:
    cycles = combined_data[topology]
    print(f"{topology:<15} {cycles.mean():<15.2f} {cycles.median():<15.2f} {cycles.min():<15.2f} {cycles.max():<15.2f}")

# Calculate cycles per node (all have 64 nodes)
print("\nCycles per Node:")
print(f"{'Topology':<15} {'Mean Cycles/Node':<15} {'Median Cycles/Node':<15}")
print("-" * 60)
for topology in ['Torus(8, 8)', 'Grid(8, 8)', 'Dense(64)']:
    cycles = combined_data[topology]
    print(f"{topology:<15} {cycles.mean()/64:<15.2f} {cycles.median()/64:<15.2f}") 