# Markdown Tests

Markdown-driven specification tests for the Destack type checker.

## Directory Structure

```
mdtest/
├── basics/         # literals, primitives, type inference
├── operators/      # binary and unary operators
├── declarations/   # variables, functions, types
└── staging/        # tests for unimplemented features
```

Each directory has its own README.md explaining what it covers.

## Test Format

Tests follow the ezno-style markdown format:

```markdown
## Section Name

### Test Name

> Optional description

```ds
const x: string = 5
```

- expected error message
```

### Sections (H2)

H2 headings define test sections within a file.

### Test Cases (H3/H4)

H3 or H4 headings define individual test cases with:
1. A descriptive name
2. A `ts`/`ds` code block
3. Optional bulleted list of expected errors

### Expected Errors

- Bullet list after code block specifies expected errors
- Error matching is case-insensitive with substring matching
- No bullet list = expect no errors

## Running Tests

```bash
# run all mdtests
cargo test --test mdtest

# filter by path/name
cargo test --test mdtest -- basics
cargo test --test mdtest -- declarations/basic

# list tests
cargo test --test mdtest -- --list

# verbose output
cargo test --test mdtest -- --verbose
```

## Adding Tests

1. Find or create the appropriate category directory
2. Find or create an appropriate `.md` file
3. Add an H3/H4 test case with code and expected errors
4. Run `cargo test --test mdtest` to verify

For bug fixes, include the issue number:
```markdown
### regression for issue #123
```
