#!/usr/bin/env python3
"""Generate type checker stress test fixtures.

Tests type inference, subtyping, and semantic analysis at scale.
"""

from pathlib import Path

OUT_DIR = Path(__file__).parent


def generate_many_types():
    """Generate a file with 100,000 type definitions."""
    print("generating many_types.ds...")
    with open(OUT_DIR / "many_types.ds", "w") as f:
        for i in range(100_000):
            f.write(f"type Type_{i} = {{ field_{i}: int32 }};\n")


def generate_deep_inheritance():
    """Generate a deep class inheritance chain."""
    print("generating deep_inheritance.ds...")
    with open(OUT_DIR / "deep_inheritance.ds", "w") as f:
        f.write("class Base { value: int32 = 0 }\n")
        for i in range(200):
            parent = "Base" if i == 0 else f"Level{i-1}"
            f.write(f"class Level{i} extends {parent} {{ level{i}: int32 = {i} }}\n")
        f.write("const instance = new Level199();\n")


def generate_large_union():
    """Generate a very large union type."""
    print("generating large_union.ds...")
    with open(OUT_DIR / "large_union.ds", "w") as f:
        types = " | ".join(f'"{i}"' for i in range(1000))
        f.write(f"type BigUnion = {types};\n")
        f.write('const x: BigUnion = "500";\n')


def generate_many_generics():
    """Generate many generic instantiations."""
    print("generating many_generics.ds...")
    with open(OUT_DIR / "many_generics.ds", "w") as f:
        f.write("struct Container<T> { value: T }\n")
        f.write("function wrap<T>(x: T): Container<T> { Container { value: x } }\n\n")
        for i in range(500):
            f.write(f"const c{i} = wrap({i});\n")
        f.write("const nested = wrap(wrap(wrap(wrap(wrap(1)))));\n")


def generate_nested_generics():
    """Generate deeply nested generic types."""
    print("generating nested_generics.ds...")
    with open(OUT_DIR / "nested_generics.ds", "w") as f:
        f.write("struct Box<T> { value: T }\n")
        # Box<Box<Box<...Box<int32>...>>>
        depth = 50
        inner = "int32"
        for _ in range(depth):
            inner = f"Box<{inner}>"
        f.write(f"type DeepBox = {inner};\n")


def generate_wide_struct():
    """Generate a struct with many fields."""
    print("generating wide_struct.ds...")
    with open(OUT_DIR / "wide_struct.ds", "w") as f:
        f.write("struct WideStruct {\n")
        for i in range(1000):
            f.write(f"    field_{i}: int32,\n")
        f.write("}\n")
        fields = ", ".join(f"field_{i}: {i}" for i in range(1000))
        f.write(f"const wide = WideStruct {{ {fields} }};\n")


def generate_many_scopes():
    """Generate deeply nested scopes with shadowing."""
    print("generating many_scopes.ds...")
    with open(OUT_DIR / "many_scopes.ds", "w") as f:
        f.write("function scopeTest(): int32 {\n")
        f.write("    const x = 0;\n")
        for i in range(200):
            f.write("    " * (i + 1) + "{\n")
            f.write("    " * (i + 2) + f"const x = {i + 1};\n")
        f.write("    " * 202 + "x\n")
        for i in range(199, -1, -1):
            f.write("    " * (i + 1) + "}\n")
        f.write("}\n")


def generate_many_overloads():
    """Generate many function overloads."""
    print("generating many_overloads.ds...")
    with open(OUT_DIR / "many_overloads.ds", "w") as f:
        # interface with many overloaded methods
        f.write("interface Overloaded {\n")
        for i in range(100):
            params = ", ".join(f"p{j}: int32" for j in range(i + 1))
            f.write(f"    call({params}): int32;\n")
        f.write("}\n")


def generate_inference_chain():
    """Generate chained type inference."""
    print("generating inference_chain.ds...")
    with open(OUT_DIR / "inference_chain.ds", "w") as f:
        # chain of inferred types
        f.write("const a = 1;\n")
        for i in range(500):
            prev = "a" if i == 0 else f"v{i-1}"
            f.write(f"const v{i} = {prev} + 1;\n")

        # nested object literals requiring inference
        f.write("\nconst nested = {\n")
        for i in range(100):
            f.write(f"    level{i}: {{\n")
            f.write(f"        value: {i},\n")
            f.write(f"        nested: {{ inner: {i * 2} }},\n")
            f.write(f"    }},\n")
        f.write("};\n")


def generate_many_implements():
    """Generate a class implementing many interfaces."""
    print("generating many_implements.ds...")
    with open(OUT_DIR / "many_implements.ds", "w") as f:
        # define 100 interfaces
        for i in range(100):
            f.write(f"interface I{i} {{ method{i}(): int32; }}\n")

        # one class implements all of them
        interfaces = ", ".join(f"I{i}" for i in range(100))
        f.write(f"\nclass ManyImpl implements {interfaces} {{\n")
        for i in range(100):
            f.write(f"    method{i}(): int32 {{ {i} }}\n")
        f.write("}\n")


def generate_wide_class():
    """Generate a class with many methods."""
    print("generating wide_class.ds...")
    with open(OUT_DIR / "wide_class.ds", "w") as f:
        f.write("class WideClass {\n")
        for i in range(500):
            f.write(f"    field_{i}: int32 = {i};\n")
        for i in range(500):
            f.write(f"    method_{i}(): int32 {{ this.field_{i} }}\n")
        f.write("}\n")
        f.write("const instance = new WideClass();\n")


def generate_wide_interface():
    """Generate an interface with many distinct methods."""
    print("generating wide_interface.ds...")
    with open(OUT_DIR / "wide_interface.ds", "w") as f:
        f.write("interface WideInterface {\n")
        for i in range(500):
            f.write(f"    method_{i}(x: int32): int32;\n")
        f.write("}\n")


def generate_deep_recursive_type():
    """Generate deeply recursive type definitions."""
    print("generating deep_recursive_type.ds...")
    with open(OUT_DIR / "deep_recursive_type.ds", "w") as f:
        # linked list type
        f.write("type List<T> = { head: T, tail: List<T> | null };\n")
        f.write("type Tree<T> = { value: T, left: Tree<T> | null, right: Tree<T> | null };\n")
        # instantiate at various depths
        for i in range(50):
            f.write(f"type ListOf{i} = List<int32>;\n")
            f.write(f"type TreeOf{i} = Tree<string>;\n")


def generate_deep_newtype():
    """Generate deep type alias chains."""
    print("generating deep_newtype.ds...")
    with open(OUT_DIR / "deep_newtype.ds", "w") as f:
        f.write("type Base = int32;\n")
        for i in range(500):
            prev = "Base" if i == 0 else f"Alias{i-1}"
            f.write(f"type Alias{i} = {prev};\n")
        f.write("const x: Alias499 = 42;\n")


def main():
    generate_many_types()
    generate_deep_inheritance()
    generate_large_union()
    generate_many_generics()
    generate_nested_generics()
    generate_wide_struct()
    generate_wide_class()
    generate_wide_interface()
    generate_many_scopes()
    generate_many_overloads()
    generate_inference_chain()
    generate_many_implements()
    generate_deep_recursive_type()
    generate_deep_newtype()
    print("done")


if __name__ == "__main__":
    main()
