# Markdown Tests

Markdown driven specification tests for the Destack type checker.
The directory layout follows the analyzer pipeline so tests live near the semantics they exercise.

## Layout Principles

Tests are grouped by semantic subsystem rather than by syntax alone.
The core buckets mirror bind, resolve, type, and flow concerns, with modules as a first class boundary.

### Primary Directories

`modules` covers import graphs, exports, globals, and namespace behavior across files.
`declarations` covers how symbols are introduced and validated across functions, classes, inheritance, static if, and reflection.
`resolution` covers name, member, call, extension, operator, and overload selection.
`types` covers assignability, generics, type operators, widening commitments, and Destack specific type system extensions.
`flow` covers narrowing and control flow constructs that commit or preserve type information.
`expressions` covers expression typing rules that are not primarily about control flow narrowing.
`options` covers compiler options that gate behavior.
`regression` covers surgical bug reproductions that do not fit cleanly elsewhere.

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
- Error matching is exact by default (after normalization)
- Prefix a line with `contains:` to match by substring instead
- Normalization: trim, lowercase, collapse whitespace runs to single spaces
- No bullet list = expect no errors

Examples:

```markdown
- member 'nonexistent' does not exist on type point
- contains: does not exist
```

## Running Tests

```bash
# run all specification tests
cargo test --test specification

# filter by path/name
cargo test --test specification -- types/widening
cargo test --test specification -- resolution/overloads

# list tests
cargo test --test specification -- --list

# verbose output
cargo test --test specification -- --verbose
```

## Adding Tests

1. Choose the semantic subsystem that owns the behavior.
2. Add the test to an existing topical file or create a new noun named file.
3. Keep positive and negative expectations in separate test cases because error assertions are not line specific.
4. Run `cargo test --test specification` to verify.

For bug fixes, include the issue number and add to `regression/`:
```markdown
### regression for issue #123
```
