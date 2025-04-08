import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np

# Read the data
df = pd.read_csv('../logs/tests-Grid(10, 10)-1000000.csv')

# Set the style
plt.style.use('default')
sns.set_theme()

# 1. Distribution of Simulated Cycles
plt.figure(figsize=(12, 8))
sns.histplot(data=df, x='Simulated Cycles', bins=30)
plt.title('Distribution of Simulated Cycles', fontsize=14)
plt.xlabel('Number of Cycles', fontsize=12)
plt.ylabel('Count', fontsize=12)
plt.savefig('plots/simulated_cycles_dist.png', dpi=300, bbox_inches='tight')
plt.close()

# 2. Busy vs Idle Cycles
plt.figure(figsize=(12, 8))
busy_idle = pd.DataFrame({
    'Type': ['Busy'] * len(df) + ['Idle'] * len(df),
    'Cycles': df['Cycles Busy'].tolist() + df['Cycles Idle'].tolist()
})
sns.boxplot(data=busy_idle, x='Type', y='Cycles')
plt.title('Distribution of Busy vs Idle Cycles', fontsize=14)
plt.ylabel('Number of Cycles', fontsize=12)
plt.xlabel('Cycle Type', fontsize=12)
plt.savefig('plots/busy_idle_dist.png', dpi=300, bbox_inches='tight')
plt.close()

# 3. Efficiency Plot (Busy Cycles / Total Cycles)
plt.figure(figsize=(12, 8))
df['Efficiency'] = df['Cycles Busy'] / (df['Cycles Busy'] + df['Cycles Idle'])
sns.histplot(data=df, x='Efficiency', bins=30)
plt.title('Distribution of Solver Efficiency', fontsize=14)
plt.xlabel('Efficiency (Busy/Total Cycles)', fontsize=12)
plt.ylabel('Count', fontsize=12)
plt.savefig('plots/efficiency_dist.png', dpi=300, bbox_inches='tight')
plt.close()

# 4. Scatter plot of Simulated Cycles vs Efficiency
plt.figure(figsize=(12, 8))
sns.scatterplot(data=df, x='Simulated Cycles', y='Efficiency', alpha=0.6)
plt.title('Simulated Cycles vs Efficiency', fontsize=14)
plt.xlabel('Number of Simulated Cycles', fontsize=12)
plt.ylabel('Efficiency (Busy/Total Cycles)', fontsize=12)
plt.savefig('plots/cycles_vs_efficiency.png', dpi=300, bbox_inches='tight')
plt.close()

# Additional plot: Test Path vs Simulated Cycles
plt.figure(figsize=(15, 8))
df['Test Name'] = df['Test Path'].apply(lambda x: x.split('/')[-1])
sns.barplot(data=df.sort_values('Simulated Cycles', ascending=False).head(20), 
           x='Test Name', y='Simulated Cycles')
plt.xticks(rotation=45, ha='right')
plt.title('Top 20 Most Time-Consuming Tests', fontsize=14)
plt.xlabel('Test Name', fontsize=12)
plt.ylabel('Number of Simulated Cycles', fontsize=12)
plt.savefig('plots/top_20_tests.png', dpi=300, bbox_inches='tight')
plt.close()

# Print statistics
print("\nPerformance Statistics:")
print("-" * 50)
print(f"Average Simulated Cycles: {df['Simulated Cycles'].mean():,.2f}")
print(f"Median Simulated Cycles: {df['Simulated Cycles'].median():,.2f}")
print(f"Average Efficiency: {df['Efficiency'].mean():.2%}")
print(f"Median Efficiency: {df['Efficiency'].median():.2%}")
print(f"\nTest Results:")
print("-" * 50)
print(f"Total Tests: {len(df)}")
print(f"All tests passed: {df['Expected Result'].equals(df['Simulated Result'])}")