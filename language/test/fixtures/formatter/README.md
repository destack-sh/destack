# Formatter Test Fixtures

Formatter fixtures use targeted transform cases and idempotent roundtrip files.

## Transform Tests (`.md` files)

Transform fixtures use MDTest cases with input code and the expected formatted output.
Cases should be organized by syntax family first and by formatting contract second.
Headings should name the behavior, not the source language mode.
Use `control` for branch, loop, try, match, and other flow constructs.
Use `patterns` for destructuring and pattern matching.
Use `types` for type syntax and type comment ownership.
Use `integration` only for cross-family cases that do not belong to one formatter family.

````markdown
## Section Name

### test name

```ds
input code here (possibly messy)
```

```ds expected
expected formatted output
```
````


## Roundtrip Tests (`roundtrip/*.{ds,d.ds,js,jsx,ts,tsx,d.ts}`)

Pre-formatted code that should remain unchanged when formatted.
Roundtrip fixtures cover canonicalized shared JS, JSX, TS, and TSX code as well as reviewed Destack local syntax.

## Running Tests

Run these commands from `language/`.

```bash
just test-formatter
```

To refresh external formatter fixtures:

```bash
python3 ./test/fixtures/conformance/fetch-suite.py ./test/fixtures/conformance/oxfmt
```

To run a specific test:

```bash
cargo test --release --test formatter -- spacing  # runs tests matching "spacing"
```
