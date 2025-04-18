import pandas as pd

df_path = "logs/satlib-Grid(8, 8)-64-100.csv"
df = pd.read_csv(df_path, sep=",", header=0)
df["Number of Variables"] = 50
df.to_csv("test.csv", sep=",", index=False)