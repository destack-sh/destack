#!/usr/bin/env python3
"""Generate resolver stress test fixtures.

Tests module resolution with many files and deep import chains.
"""

from pathlib import Path

OUT_DIR = Path(__file__).parent


def generate_many_modules():
    """Generate a project with 5,000 module files."""
    print("generating many_modules/...")
    modules_dir = OUT_DIR / "many_modules"
    modules_dir.mkdir(exist_ok=True)

    for i in range(5000):
        with open(modules_dir / f"module_{i}.ds", "w") as f:
            f.write(f"export const value_{i}: int32 = {i};\n")
            f.write(f"export function compute_{i}(x: int32): int32 {{ x + {i} }}\n")

    # index file importing all modules
    with open(modules_dir / "index.ds", "w") as f:
        for i in range(5000):
            f.write(f'import {{ value_{i} }} from "./module_{i}";\n')
        f.write("\nexport const total = " + " + ".join(f"value_{i}" for i in range(100)) + ";\n")


def generate_deep_imports():
    """Generate a project with 100-deep import chain."""
    print("generating deep_imports/...")
    imports_dir = OUT_DIR / "deep_imports"
    imports_dir.mkdir(exist_ok=True)

    depth = 100
    for i in range(depth):
        with open(imports_dir / f"level_{i}.ds", "w") as f:
            if i == 0:
                f.write("export const base: int32 = 1;\n")
            else:
                f.write(f'import {{ base as prev }} from "./level_{i-1}";\n')
                f.write(f"export const base: int32 = prev + 1;\n")

    with open(imports_dir / "index.ds", "w") as f:
        f.write(f'import {{ base }} from "./level_{depth-1}";\n')
        f.write("export const result = base;\n")


def generate_circular_stress():
    """Generate a project with complex (but valid) circular dependencies."""
    print("generating circular_stress/...")
    circular_dir = OUT_DIR / "circular_stress"
    circular_dir.mkdir(exist_ok=True)

    # create a web of interdependent modules
    num_modules = 50
    for i in range(num_modules):
        with open(circular_dir / f"module_{i}.ds", "w") as f:
            f.write(f"export const id_{i}: int32 = {i};\n")
            # import from a few other modules (wrapping around)
            for j in range(3):
                other = (i + j + 1) % num_modules
                if other != i:
                    f.write(f'import {{ id_{other} }} from "./module_{other}";\n')
            f.write(f"export const sum_{i} = id_{i};\n")

    with open(circular_dir / "index.ds", "w") as f:
        for i in range(num_modules):
            f.write(f'import {{ sum_{i} }} from "./module_{i}";\n')
        f.write("export const total = " + " + ".join(f"sum_{i}" for i in range(num_modules)) + ";\n")


def generate_wide_imports():
    """Generate a single file that imports from many modules."""
    print("generating wide_imports/...")
    wide_dir = OUT_DIR / "wide_imports"
    wide_dir.mkdir(exist_ok=True)

    # create 1000 small modules
    for i in range(1000):
        with open(wide_dir / f"dep_{i}.ds", "w") as f:
            f.write(f"export const val_{i}: int32 = {i};\n")

    # one file imports all of them
    with open(wide_dir / "index.ds", "w") as f:
        for i in range(1000):
            f.write(f'import {{ val_{i} }} from "./dep_{i}";\n')
        f.write("\nexport const sum = " + " + ".join(f"val_{i}" for i in range(1000)) + ";\n")


def main():
    generate_many_modules()
    generate_deep_imports()
    generate_circular_stress()
    generate_wide_imports()
    print("done")


if __name__ == "__main__":
    main()
