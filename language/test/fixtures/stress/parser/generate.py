#!/usr/bin/env python3
"""Generate parser stress test fixtures.

Tests lexer and parser throughput, depth limits, and error recovery.
"""

from pathlib import Path

OUT_DIR = Path(__file__).parent


# --- scale tests ---

def generate_100k_lines():
    """Generate a file with 100,000 variable declarations."""
    print("generating 100k_lines.ds...")
    with open(OUT_DIR / "100k_lines.ds", "w") as f:
        for i in range(100_000):
            f.write(f"const var_{i}: int32 = {i};\n")


def generate_many_functions():
    """Generate a file with 10,000 function definitions."""
    print("generating many_functions.ds...")
    with open(OUT_DIR / "many_functions.ds", "w") as f:
        for i in range(10_000):
            f.write(f"function func_{i}(x: int32): int32 {{ x + {i} }}\n")


def generate_many_locals():
    """Generate function with many local variables."""
    print("generating many_locals.ds...")
    with open(OUT_DIR / "many_locals.ds", "w") as f:
        f.write("function manyLocals(): int32 {\n")
        for i in range(5000):
            f.write(f"    const local_{i} = {i};\n")
        f.write("    local_0 + local_4999\n")
        f.write("}\n")


def generate_many_parameters():
    """Generate functions with many parameters."""
    print("generating many_parameters.ds...")
    with open(OUT_DIR / "many_parameters.ds", "w") as f:
        params = ", ".join(f"p{i}: int32" for i in range(500))
        args = ", ".join(f"p{i}" for i in range(500))
        f.write(f"function manyParams({params}): int32 {{ {args.replace(', ', ' + ')} }}\n")
        call_args = ", ".join(str(i) for i in range(500))
        f.write(f"const result = manyParams({call_args});\n")


# --- depth tests ---

def generate_deep_nesting():
    """Generate a file with 500 levels of nested blocks."""
    print("generating deep_nesting.ds...")
    with open(OUT_DIR / "deep_nesting.ds", "w") as f:
        depth = 500
        f.write("function deepNest(): int32 {\n")
        for i in range(depth):
            f.write("    " * (i + 1) + "{\n")
        f.write("    " * (depth + 1) + "42\n")
        for i in range(depth - 1, -1, -1):
            f.write("    " * (i + 1) + "}\n")
        f.write("}\n")


def generate_deep_expressions():
    """Generate deeply nested expressions."""
    print("generating deep_expressions.ds...")
    with open(OUT_DIR / "deep_expressions.ds", "w") as f:
        # nested parentheses
        depth = 200
        f.write("const nested = " + "(" * depth + "1" + ")" * depth + ";\n")
        # nested ternaries
        f.write("const ternary = ")
        for i in range(100):
            f.write(f"true ? {i} : ")
        f.write("0;\n")


def generate_chained_calls():
    """Generate deeply chained method calls."""
    print("generating chained_calls.ds...")
    with open(OUT_DIR / "chained_calls.ds", "w") as f:
        f.write("struct Builder { value: int32 }\n")
        f.write("extension of Builder {\n")
        f.write("    add(n: int32): Builder { Builder { value: this.value + n } }\n")
        f.write("}\n")
        f.write("const built = Builder { value: 0 }")
        for i in range(500):
            f.write(f".add({i})")
        f.write(";\n")


def generate_operator_chain():
    """Generate long operator chains (precedence parsing stress)."""
    print("generating operator_chain.ds...")
    with open(OUT_DIR / "operator_chain.ds", "w") as f:
        ops = ["+", "-", "*", "/", "%", "&&", "||", "|", "&", "^"]
        chain = "1"
        for i in range(5000):
            chain += f" {ops[i % len(ops)]} {i}"
        f.write(f"const x = {chain};\n")


# --- width tests ---

def generate_long_lines():
    """Generate a file with very long lines."""
    print("generating long_lines.ds...")
    with open(OUT_DIR / "long_lines.ds", "w") as f:
        # single line with 10k tokens
        f.write("const sum = " + " + ".join(["1"] * 10_000) + ";\n")
        # single line with 50k characters
        f.write(f'const longLine = "{("x" * 50_000)}";\n')


def generate_long_identifiers():
    """Generate a file with identifiers up to 10KB in length."""
    print("generating long_identifiers.ds...")
    with open(OUT_DIR / "long_identifiers.ds", "w") as f:
        for length in [100, 1000, 5000, 10000]:
            name = "x" * length
            f.write(f"const {name}: int32 = 1;\n")


def generate_large_literals():
    """Generate a file with large string and array literals."""
    print("generating large_literals.ds...")
    with open(OUT_DIR / "large_literals.ds", "w") as f:
        big_string = "a" * (1024 * 1024)
        f.write(f'const bigString: string = "{big_string}";\n')
        f.write(f"const bigArray: int32[] = [{', '.join(str(i) for i in range(10_000))}];\n")


# --- unicode tests ---

def generate_unicode():
    """Generate file with unicode edge cases."""
    print("generating unicode.ds...")
    with open(OUT_DIR / "unicode.ds", "w") as f:
        f.write("const \u03b1\u03b2\u03b3 = 1;\n")  # greek
        f.write("const \u4e2d\u6587 = 2;\n")  # chinese
        f.write("const \u0430\u0431\u0432 = 3;\n")  # cyrillic
        long_unicode = "a\u0301" * 1000
        f.write(f"const {long_unicode} = 4;\n")
        f.write('const emoji = "Hello 🌍🚀✨ World";\n')


# --- edge cases ---

def generate_edge_cases():
    """Generate parser edge case files."""
    print("generating edge cases...")

    # empty file
    with open(OUT_DIR / "empty.ds", "w") as f:
        pass

    # whitespace only
    with open(OUT_DIR / "whitespace_only.ds", "w") as f:
        f.write("   \n\n\t\t\n   \n")

    # comments only
    with open(OUT_DIR / "comments_only.ds", "w") as f:
        for i in range(1000):
            f.write(f"// comment {i}\n")

    # no trailing newline
    with open(OUT_DIR / "no_trailing_newline.ds", "w") as f:
        f.write("const x = 1;")

    # CRLF endings
    with open(OUT_DIR / "crlf_endings.ds", "wb") as f:
        for i in range(100):
            f.write(f"const x{i} = {i};\r\n".encode())

    # BOM
    with open(OUT_DIR / "bom.ds", "wb") as f:
        f.write(b"\xef\xbb\xbf")
        f.write("const x = 1;\n".encode())

    # very long comment
    with open(OUT_DIR / "long_comment.ds", "w") as f:
        f.write("/* " + "x" * (1024 * 1024) + " */\n")
        f.write("const x = 1;\n")


# --- pathological (error recovery) ---

def generate_pathological():
    """Generate pathological inputs for error recovery testing."""
    print("generating pathological cases...")

    # unclosed delimiters
    with open(OUT_DIR / "unclosed_parens.ds", "w") as f:
        f.write("const x = " + "(" * 10000 + "1;\n")

    with open(OUT_DIR / "unclosed_braces.ds", "w") as f:
        f.write("function f() " + "{" * 5000 + "\n")

    with open(OUT_DIR / "unclosed_brackets.ds", "w") as f:
        f.write("const x = " + "[" * 10000 + "1;\n")

    # unmatched closing delimiters
    with open(OUT_DIR / "unmatched_close_parens.ds", "w") as f:
        f.write("const x = 1" + ")" * 10000 + ";\n")

    with open(OUT_DIR / "unmatched_close_braces.ds", "w") as f:
        f.write("const x = 1;" + "}" * 5000 + "\n")

    with open(OUT_DIR / "unmatched_close_brackets.ds", "w") as f:
        f.write("const x = 1" + "]" * 10000 + ";\n")

    # bracket spam
    import random
    random.seed(42)
    with open(OUT_DIR / "bracket_spam.ds", "w") as f:
        brackets = "()[]{}<>"
        spam = "".join(random.choice(brackets) for _ in range(50000))
        f.write(f"const x = {spam};\n")

    # nested ternary (deep)
    with open(OUT_DIR / "nested_ternary.ds", "w") as f:
        depth = 1000
        expr = "true ? " * depth + "1" + " : 0" * depth
        f.write(f"const x = {expr};\n")

    # nested templates
    with open(OUT_DIR / "nested_templates.ds", "w") as f:
        depth = 500
        f.write("type T = " + "Array<" * depth + "int32" + ">" * depth + ";\n")

    # many string escapes
    with open(OUT_DIR / "many_escapes.ds", "w") as f:
        escapes = '\\n\\t\\r\\"\\\\' * 10000
        f.write(f'const s = "{escapes}";\n')

    # only errors
    with open(OUT_DIR / "only_errors.ds", "w") as f:
        for i in range(1000):
            f.write(f"@@@ $$$ %%% {i}\n")

    # almost valid (typos)
    with open(OUT_DIR / "almost_valid.ds", "w") as f:
        for i in range(1000):
            f.write(f"cosnt x{i} = {i}\n")
            f.write(f"functoin f{i}() {{}}\n")


def main():
    generate_100k_lines()
    generate_many_functions()
    generate_many_locals()
    generate_many_parameters()
    generate_deep_nesting()
    generate_deep_expressions()
    generate_chained_calls()
    generate_operator_chain()
    generate_long_lines()
    generate_long_identifiers()
    generate_large_literals()
    generate_unicode()
    generate_edge_cases()
    generate_pathological()
    print("done")


if __name__ == "__main__":
    main()
