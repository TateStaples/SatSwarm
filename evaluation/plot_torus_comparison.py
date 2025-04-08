import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import os

# Define the data files
data_dir = 'data'
torus_2_2_file = os.path.join(data_dir, 'eval_set-Torus(2, 2)-8-1-1000000.csv')
torus_8_8_file = os.path.join(data_dir, 'eval_set-Torus(8, 8)-64-1-1000000.csv')
torus_12_12_file = os.path.join(data_dir, 'eval_set-Torus(12, 12)-144-1-1000000.csv')
torus_18_18_file = os.path.join(data_dir, 'eval_set-Torus(18, 18)-324-1-1000000.csv')

# Read the data
torus_2_2_data = pd.read_csv(torus_2_2_file)
torus_8_8_data = pd.read_csv(torus_8_8_file)
torus_12_12_data = pd.read_csv(torus_12_12_file)
torus_18_18_data = pd.read_csv(torus_18_18_file)

# Extract test names and simulation cycles
def extract_test_name(path):
    return path.split('/')[-1]

torus_2_2_data['Test'] = torus_2_2_data['Test Path'].apply(extract_test_name)
torus_8_8_data['Test'] = torus_8_8_data['Test Path'].apply(extract_test_name)
torus_12_12_data['Test'] = torus_12_12_data['Test Path'].apply(extract_test_name)
torus_18_18_data['Test'] = torus_18_18_data['Test Path'].apply(extract_test_name)

# Create a combined dataframe for plotting
combined_data = pd.DataFrame({
    'Test': torus_2_2_data['Test'],
    'Torus(2, 2)': torus_2_2_data['Simulated Cycles'],
    'Torus(8, 8)': torus_8_8_data['Simulated Cycles'],
    'Torus(12, 12)': torus_12_12_data['Simulated Cycles'],
    'Torus(18, 18)': torus_18_18_data['Simulated Cycles']
})

# Sort by test name for consistent ordering
combined_data = combined_data.sort_values('Test')

# Set up the plot
plt.figure(figsize=(14, 8))
bar_width = 0.2
index = np.arange(len(combined_data))

# Create bars for each topology
plt.bar(index, combined_data['Torus(2, 2)'], bar_width, label='Torus(2, 2)', color='skyblue')
plt.bar(index + bar_width, combined_data['Torus(8, 8)'], bar_width, label='Torus(8, 8)', color='lightgreen')
plt.bar(index + 2*bar_width, combined_data['Torus(12, 12)'], bar_width, label='Torus(12, 12)', color='salmon')
plt.bar(index + 3*bar_width, combined_data['Torus(18, 18)'], bar_width, label='Torus(18, 18)', color='purple')

# Add labels and title
plt.xlabel('Test Case')
plt.ylabel('Simulation Cycles')
plt.title('Comparison of Simulation Cycles Across Torus Topologies')
plt.xticks(index + 1.5*bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the plot
plt.savefig('torus_comparison.png', dpi=300)
print("Plot saved as 'torus_comparison.png'")

# Also create a log-scale version for better visualization
plt.figure(figsize=(14, 8))
plt.bar(index, combined_data['Torus(2, 2)'], bar_width, label='Torus(2, 2)', color='skyblue')
plt.bar(index + bar_width, combined_data['Torus(8, 8)'], bar_width, label='Torus(8, 8)', color='lightgreen')
plt.bar(index + 2*bar_width, combined_data['Torus(12, 12)'], bar_width, label='Torus(12, 12)', color='salmon')
plt.bar(index + 3*bar_width, combined_data['Torus(18, 18)'], bar_width, label='Torus(18, 18)', color='purple')

plt.xlabel('Test Case')
plt.ylabel('Simulation Cycles (log scale)')
plt.title('Comparison of Simulation Cycles Across Torus Topologies (Log Scale)')
plt.xticks(index + 1.5*bar_width, combined_data['Test'], rotation=45, ha='right')
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')
plt.tight_layout()

# Save the log-scale plot
plt.savefig('torus_comparison_log.png', dpi=300)
print("Log-scale plot saved as 'torus_comparison_log.png'")

# Create a box plot to show the distribution of cycles
plt.figure(figsize=(10, 6))
box_data = [
    combined_data['Torus(2, 2)'],
    combined_data['Torus(8, 8)'],
    combined_data['Torus(12, 12)'],
    combined_data['Torus(18, 18)']
]
plt.boxplot(box_data, labels=['Torus(2, 2)', 'Torus(8, 8)', 'Torus(12, 12)', 'Torus(18, 18)'])
plt.ylabel('Simulation Cycles')
plt.title('Distribution of Simulation Cycles Across Torus Topologies')
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.tight_layout()

# Save the box plot
plt.savefig('torus_boxplot.png', dpi=300)
print("Box plot saved as 'torus_boxplot.png'")

# Create a box plot with log scale
plt.figure(figsize=(10, 6))
plt.boxplot(box_data, labels=['Torus(2, 2)', 'Torus(8, 8)', 'Torus(12, 12)', 'Torus(18, 18)'])
plt.ylabel('Simulation Cycles (log scale)')
plt.title('Distribution of Simulation Cycles Across Torus Topologies (Log Scale)')
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.yscale('log')
plt.tight_layout()

# Save the log-scale box plot
plt.savefig('torus_boxplot_log.png', dpi=300)
print("Log-scale box plot saved as 'torus_boxplot_log.png'")

# Print summary statistics
print("\nSummary Statistics:")
print(f"{'Topology':<15} {'Mean Cycles':<15} {'Median Cycles':<15} {'Min Cycles':<15} {'Max Cycles':<15} {'Num Nodes':<15}")
print("-" * 90)
for topology in ['Torus(2, 2)', 'Torus(8, 8)', 'Torus(12, 12)', 'Torus(18, 18)']:
    cycles = combined_data[topology]
    num_nodes = {
        'Torus(2, 2)': 8,
        'Torus(8, 8)': 64,
        'Torus(12, 12)': 144,
        'Torus(18, 18)': 324
    }
    print(f"{topology:<15} {cycles.mean():<15.2f} {cycles.median():<15.2f} {cycles.min():<15.2f} {cycles.max():<15.2f} {num_nodes[topology]:<15}")

# Calculate cycles per node
print("\nCycles per Node:")
print(f"{'Topology':<15} {'Mean Cycles/Node':<15} {'Median Cycles/Node':<15}")
print("-" * 60)
for topology in ['Torus(2, 2)', 'Torus(8, 8)', 'Torus(12, 12)', 'Torus(18, 18)']:
    cycles = combined_data[topology]
    num_nodes = {
        'Torus(2, 2)': 8,
        'Torus(8, 8)': 64,
        'Torus(12, 12)': 144,
        'Torus(18, 18)': 324
    }
    print(f"{topology:<15} {cycles.mean()/num_nodes[topology]:<15.2f} {cycles.median()/num_nodes[topology]:<15.2f}") 