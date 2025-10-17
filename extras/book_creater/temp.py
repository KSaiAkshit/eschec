
import numpy as np
import seaborn as sns
import matplotlib.pyplot as plt

# --- 1. Data Setup (Using your last 8x8 matrix) ---
data = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25, -8, -25, -2, -9, -25, -24, -52,
    -24, -20, 10, 9, -1, -9, -19, -41,
    -17, 3, 22, 22, 22, 11, 8, -18,
    -18, -6, 16, 25, 16, 17, 4, -18,
    -23, -3, -1, 15, 10, -3, -20, -22,
    -42, -20, -10, -5, -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64
]
matrix = np.array(data).reshape(8, 8)

# --- 2. List of Colormaps (Cmaps) ---
# Matplotlib has many built-in colormaps, categorized below.
# You can use any of these strings for the 'cmap' argument.

print("--- AVAILABLE COLORMAP CATEGORIES (use the string name) ---")

# A. Perceptually Uniform Sequential (Good for general data visualization)
# Recommended defaults: 'viridis', 'plasma', 'inferno', 'magma', 'cividis'
print("\nA. Sequential: ", ['viridis', 'plasma', 'inferno', 'magma', 'cividis', 'Greens', 'Blues', 'Oranges', 'Reds', 'PuBu', 'GnBu', 'YlOrBr', 'gray'])

# B. Diverging (Good for data that deviates around a central value, like 0)
# Recommended defaults: 'coolwarm', 'seismic', 'RdBu'
print("\nB. Diverging: ", ['coolwarm', 'seismic', 'RdBu', 'PiYG', 'bwr', 'PRGn'])

# C. Qualitative (Best for discrete or categorical data - less common for heatmaps)
# Recommended defaults: 'tab10', 'Set1', 'Pastel1'
print("\nC. Qualitative: ", ['tab10', 'Set1', 'Pastel1', 'Dark2', 'Accent'])

# D. Miscellaneous (Sometimes useful for specific effects)
print("\nD. Miscellaneous: ", ['jet', 'hsv', 'twilight', 'ocean', 'terrain', 'gist_earth'])


# --- 3. Example Heatmap Function ---
def create_heatmap(data_matrix, cmap_name, title):
    """Generates and saves a heatmap with a specified colormap."""
    plt.figure(figsize=(8, 6))
    sns.heatmap(
        data_matrix,
        annot=True,
        fmt="d",
        cmap=cmap_name, # <-- This is where the colormap is set
        linewidths=.5,
        cbar_kws={'label': 'Value'}
    )
    plt.title(title)
    plt.savefig(f'heatmap_{cmap_name}.png')
    plt.close()
    print(f"Generated heatmap with '{cmap_name}' colormap.")


# --- 4. Running Examples ---

# Example 1: Perceptually Uniform Sequential (Default and excellent for general use)
create_heatmap(matrix, 'viridis', 'Heatmap (Colormap: viridis - Sequential)')

# Example 2: Diverging (Good for showing positive/negative relative to zero)
create_heatmap(matrix, 'coolwarm', 'Heatmap (Colormap: coolwarm - Diverging)')

# Example 3: Different Sequential (A classic but non-uniform color progression)
create_heatmap(matrix, 'hot', 'Heatmap (Colormap: hot - Sequential)')

print("\nScript finished. Check the output directory for the generated image files.")
