# Extract Function

## Local Expression

### Extracts an expression inside a function

Extract should create a nested function and replace the selected expression with a call.

The expression is selected via a caret marker.

```ds:main.ds
function main(): int32 {
    const a = 1;
    const b = 2;
    const total = a + b;
    //              ^^^^^ selection
    return total;
}
```

```query extract_function selection compute_total
```

```expected:main.ds
function main(): int32 {
    const a = 1;
    const b = 2;

    function compute_total(a: int32, b: int32): int32 {
        return a + b;
    }

    const total = compute_total(a, b);
    return total;
}
```

## Top-Level Expression

### Extracts an expression at the module scope

Extract should insert a top-level function when the selection is at the module scope.

```ds:main.ds
const total = 3 * 4;
//              ^^^^^ selection
```

```query extract_function selection compute_total
```

```expected:main.ds
function compute_total(): int32 {
    return 3 * 4;
}

const total = compute_total();
```

## Return Expression

### Extracts an expression from a return statement

Extract should replace a selected return expression with a function call.

```ds:main.ds
function main(): int32 {
    return 1 + 2;
    //       ^^^^^ selection
}
```

```query extract_function selection compute_total
```

```expected:main.ds
function main(): int32 {

    function compute_total(): int32 {
        return 1 + 2;
    }

    return compute_total();
}
```

## Expression With Free Variables

### Extracts parameters from referenced bindings

Extract should add parameters for referenced bindings outside the selection.

```ds:main.ds
const base = 10;
const total = base + 2;
//              ^^^^^^^ selection
```

```query extract_function selection compute_total
```

```expected:main.ds
const base = 10;

function compute_total(base: int32): int32 {
    return base + 2;
}

const total = compute_total(base);
```

### Extracts expressions with member access and free variables

Extract should preserve member access while turning the outer bindings into parameters.

```ds:main.ds
type User = {
    name: string,
};

function main(user: User, suffix: string): string {
    const label = user.name + suffix;
    //              ^^^^^^^^^^^^^^^^^^ selection
    return label;
}
```

```query extract_function selection format_label
```

```expected:main.ds
type User = {
    name: string,
};

function main(user: User, suffix: string): string {

    function format_label(user: User, suffix: string): string {
        return user.name + suffix;
    }

    const label = format_label(user, suffix);
    return label;
}
```

## Statement Block

### Extracts statements and returns the produced value

Extract should return values that are defined in the selection and used after it.

```ds:main.ds
function main(): int32 {
    const a = 1; const b = 2; const c = a + b;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return c;
}
```

```query extract_function selection compute_total
```

```expected:main.ds
function main(): int32 {

    function compute_total(): int32 {
        const a = 1; const b = 2; const c = a + b;
        return c;
    }

    const c = compute_total();
    return c;
}
```

## Tuple Output

### Returns multiple outputs as a tuple

Extract should bundle multiple outputs into a tuple and destructure at the call site.

```ds:main.ds
function main(): int32 {
    const a = 1; const b = 2;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return a + b;
}
```

```query extract_function selection compute_pair
```

```expected:main.ds
function main(): int32 {

    function compute_pair(): (int32, int32) {
        const a = 1; const b = 2;
        return (a, b);
    }

    const (a, b) = compute_pair();
    return a + b;
}
```

## Statement Block Without Outputs

### Extracts statements without producing outputs

Extract should insert a call statement when no outputs are used after the selection.

```ds:main.ds
function main(): int32 {
    const base = 1;
    const temp = base + 2; const value = temp * 3;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return base + 1;
}
```

```query extract_function selection compute_values
```

```expected:main.ds
function main(): int32 {
    const base = 1;

    function compute_values(base: int32) {
        const temp = base + 2; const value = temp * 3;
    }

    compute_values(base);
    return base + 1;
}
```

## Statement Block With Free Parameter

### Extracts statements with external dependencies

Extract should include referenced bindings as parameters and return the produced values.

```ds:main.ds
function main(): int32 {
    const base = 4;
    const a = base + 1; const b = base + 2;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return a + b;
}
```

```query extract_function selection compute_pair
```

```expected:main.ds
function main(): int32 {
    const base = 4;

    function compute_pair(base: int32): (int32, int32) {
        const a = base + 1; const b = base + 2;
        return (a, b);
    }

    const (a, b) = compute_pair(base);
    return a + b;
}
```

## Control Flow Guard

### Skips extraction when control flow is inside the selection

Extract should refuse to extract statement blocks that contain control flow.

```ds:main.ds
function main(flag: boolean): int32 {
    if (flag) { return 1; } const value = 2;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return value;
}
```

```query extract_function selection compute_value
<none>
```

## Partial Statement Selection

### Skips extraction for partial statement ranges

Extract should reject selections that do not fully cover statements.

```ds:main.ds
function main(): int32 {
    const a = 1; const b = 2;
  //        ^^^^^^^^^^^^^^^^^ selection
    return a + b;
}
```

```query extract_function selection compute_partial
<none>
```

## Mutable Output

### Uses let bindings when outputs are mutable

Extract should use `let` at the call site when the output is mutable.

```ds:main.ds
function main(): int32 {
    let count = 0; count = count + 1;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return count;
}
```

```query extract_function selection increment
```

```expected:main.ds
function main(): int32 {

    function increment(): int32 {
        let count = 0; count = count + 1;
        return count;
    }

    let count = increment();
    return count;
}
```

## Async Extraction

### Propagates async when extracting awaited expressions

Extract should create an async function when the selection contains `await`.

```ds:main.ds
async function main(): Promise<int32> {
    const value = await load_value();
    //              ^^^^^^^^^^^^^^^^^ selection
    return value;
}
```

```query extract_function selection load_async
```

```expected:main.ds
async function main(): Promise<int32> {

    async function load_async() {
        return await load_value();
    }

    const value = await load_async();
    return value;
}
```

## Damaged Syntax

### Extracts valid expressions after malformed call statements

Extract should still rewrite later valid expressions after malformed call recovery.

```ds:main.ds
broken(,

function main(): int32 {
    const total = 1 + 2;
    //              ^^^^^ selection
    return total;
}
```

```query extract_function selection compute_total
```

```expected:main.ds
broken(,

function main(): int32 {

    function compute_total(): int32 {
        return 1 + 2;
    }

    const total = compute_total();
    return total;
}
```

### Extracts statement blocks after malformed call statements

Extract should still handle statement-block extraction after malformed call recovery.

```ds:main.ds
broken(,

function main(base: int32): int32 {
    const a = base + 1; const b = base + 2;
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return a + b;
}
```

```query extract_function selection compute_pair
```

```expected:main.ds
broken(,

function main(base: int32): int32 {

    function compute_pair(base: int32): (int32, int32) {
        const a = base + 1; const b = base + 2;
        return (a, b);
    }

    const (a, b) = compute_pair(base);
    return a + b;
}
```

## Control Flow Variants

### Skips extraction when selection contains break

Extract should refuse to extract statement blocks that contain `break`.

```ds:main.ds
function main(flag: boolean): int32 {
    let count = 0;
    while (flag) { count = count + 1; break; }
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return count;
}
```

```query extract_function selection count_loop
<none>
```

### Skips extraction when selection contains continue

Extract should refuse to extract statement blocks that contain `continue`.

```ds:main.ds
function main(flag: boolean): int32 {
    let count = 0;
    while (flag) { count = count + 1; continue; }
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return count;
}
```

```query extract_function selection count_loop
<none>
```

### Skips extraction when selection contains throw

Extract should refuse to extract statement blocks that contain `throw`.

```ds:main.ds
function main(flag: boolean): int32 {
    if (flag) { throw "nope"; }
  //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return 1;
}
```

```query extract_function selection throw_block
<none>
```

## Boundary Selection

### Skips extraction when selection crosses block boundaries

Extract should reject selections that start inside a block and end outside.

```ds:main.ds
function main(flag: boolean): int32 {
    if (flag) { const a = 1; } const b = 2;
  //               ^^^^^^^^^^^^^^^^^^^^^^^^^^ selection
    return b;
}
```

```query extract_function selection mixed_block
<none>
```
