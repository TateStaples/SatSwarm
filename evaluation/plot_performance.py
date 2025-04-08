import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import os
import re
from glob import glob

# Set the style
plt.style.use('default')
sns.set_theme()

# Function to extract information from filename
def extract_info_from_filename(filename):
    # Extract topology and dimensions from filename
    match = re.search(r'eval_set-([A-Za-z]+)\(([\d,]+)\)-(\d+)-\d+-\d+\.csv', filename)
    if match:
        topology, dimensions, nodes = match.groups()
        return {
            'topology': topology,
            'dimensions': dimensions,
            'nodes': int(nodes),
            'filename': filename
        }
    return None

# Get all CSV files from the data directory
data_files = glob('evaluation/data/*.csv')
all_data = []

# Process each file
for file_path in data_files:
    info = extract_info_from_filename(os.path.basename(file_path))
    if info:
        df = pd.read_csv(file_path)
        df['Topology'] = info['topology']
        df['Dimensions'] = info['dimensions']
        df['Node Count'] = info['nodes']
        all_data.append(df)

# Combine all data
combined_df = pd.concat(all_data, ignore_index=True)

# Create plots directory if it doesn't exist
os.makedirs('evaluation/plots', exist_ok=True)

# 1. Node Count vs Simulated Cycles (boxplot)
plt.figure(figsize=(12, 8))
sns.boxplot(data=combined_df, x='Node Count', y='Simulated Cycles')
plt.title('Simulated Cycles vs Node Count', fontsize=14)
plt.xlabel('Number of Nodes', fontsize=12)
plt.ylabel('Simulated Cycles', fontsize=12)
plt.savefig('evaluation/plots/node_count_vs_cycles.png', dpi=300, bbox_inches='tight')
plt.close()

# 2. Node Count vs Efficiency
plt.figure(figsize=(12, 8))
combined_df['Efficiency'] = combined_df['Cycles Busy'] / (combined_df['Cycles Busy'] + combined_df['Cycles Idle'])
sns.boxplot(data=combined_df, x='Node Count', y='Efficiency')
plt.title('Efficiency vs Node Count', fontsize=14)
plt.xlabel('Number of Nodes', fontsize=12)
plt.ylabel('Efficiency (Busy/Total Cycles)', fontsize=12)
plt.savefig('evaluation/plots/node_count_vs_efficiency.png', dpi=300, bbox_inches='tight')
plt.close()

# 3. Comparison of 64-node topologies
node_64_data = combined_df[combined_df['Node Count'] == 64]
plt.figure(figsize=(12, 8))
sns.boxplot(data=node_64_data, x='Topology', y='Simulated Cycles')
plt.title('Performance Comparison of 64-Node Topologies', fontsize=14)
plt.xlabel('Topology', fontsize=12)
plt.ylabel('Simulated Cycles', fontsize=12)
plt.savefig('evaluation/plots/64node_topology_comparison.png', dpi=300, bbox_inches='tight')
plt.close()

# 4. Efficiency comparison for 64-node topologies
plt.figure(figsize=(12, 8))
sns.boxplot(data=node_64_data, x='Topology', y='Efficiency')
plt.title('Efficiency Comparison of 64-Node Topologies', fontsize=14)
plt.xlabel('Topology', fontsize=12)
plt.ylabel('Efficiency (Busy/Total Cycles)', fontsize=12)
plt.savefig('evaluation/plots/64node_efficiency_comparison.png', dpi=300, bbox_inches='tight')
plt.close()

# Print statistics
print("\nPerformance Statistics:")
print("-" * 50)
print(f"Average Simulated Cycles: {combined_df['Simulated Cycles'].mean():,.2f}")
print(f"Median Simulated Cycles: {combined_df['Simulated Cycles'].median():,.2f}")
print(f"Average Efficiency: {combined_df['Efficiency'].mean():.2%}")
print(f"Median Efficiency: {combined_df['Efficiency'].median():.2%}")

print("\nStatistics by Node Count:")
print("-" * 50)
node_stats = combined_df.groupby('Node Count').agg({
    'Simulated Cycles': ['mean', 'median'],
    'Efficiency': ['mean', 'median']
}).round(2)
print(node_stats)

print("\nStatistics for 64-Node Topologies:")
print("-" * 50)
topology_stats = node_64_data.groupby('Topology').agg({
    'Simulated Cycles': ['mean', 'median'],
    'Efficiency': ['mean', 'median']
}).round(2)
print(topology_stats)

print(f"\nTest Results:")
print("-" * 50)
print(f"Total Tests: {len(combined_df)}")
print(f"All tests passed: {combined_df['Expected Result'].equals(combined_df['Simulated Result'])}")