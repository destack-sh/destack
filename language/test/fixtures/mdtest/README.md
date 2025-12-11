# Markdown Tests

Markdown-driven specification tests for the Destack type checker.

## Directory Structure

Tests are organized into two main sections:

### TypeScript

Standard JavaScript/TypeScript behavior that Destack inherits:

| Directory | Description |
|-----------|-------------|
| `basics/` | Primitives, special types, type inference |
| `literals/` | Literal type inference (strings, numbers, arrays, objects) |
| `operators/` | Arithmetic, comparison, logical, bitwise operators |
| `declarations/` | Variables, functions, type aliases |
| `composites/` | Unions, intersections, tuples |
| `narrowing/` | Type narrowing, control flow analysis |
| `classes/` | Class declarations, constructors, methods |
| `inheritance/` | Class extends, interface implements |
| `iterators/` | for...of, for...in, iterables |
| `modules/` | Imports, exports, resolution |
| `async/` | Promises, async/await, generators |

### Destack

Extensions beyond standard TypeScript, organized by `LanguageFeature`:

| Directory | Feature | Description |
|-----------|---------|-------------|
| `expressions/` | Expressions | Implicit returns, if-expressions, match, ranges, tuples, patterns |
| `types/` | Types | Newtypes, structs, precise primitives, where clauses, refinements |
| `reflection/` | Reflection | Type descriptors, runtime type info, decorator metadata |
| `dispatch/` | Dispatch | Extensions, function/operator overloading |
| `ownership/` | Ownership | References (`&T`), values (`^T`), mutability |

### Development

| Directory | Description |
|-----------|-------------|
| `staging/` | Tests for unimplemented features |
| `regression/` | Tests for specific bug fixes |

## Test Format

Tests use markdown with code blocks and expected errors:

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

For bug fixes, include the issue number and add to `regression/`:
```markdown
### regression for issue #123
```
