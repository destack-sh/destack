
## Local Expression

### Extract a return expression

The extracted expression becomes a binding before its statement.

```tspp main.tspp
function total(left: int32, right: int32): int32 {
    return left + right;
           ^^^^^^^^^^^^ selection
}
```

```query extract_variable main.tspp#selection new_name=sum
```

```tspp main.tspp after
function total(left: int32, right: int32): int32 {
    const sum = left + right;
    return sum;
}
```

### Extract a subexpression

A subexpression becomes a binding before its containing statement.

```tspp main.tspp
function total(left: int32, right: int32): int32 {
    return (left + right) * 2;
            ^^^^^^^^^^^^ selection
}
```

```query extract_variable main.tspp#selection new_name=sum
```

```tspp main.tspp after
function total(left: int32, right: int32): int32 {
    const sum = left + right;
    return (sum) * 2;
}
```

### Extract two local expressions

Each extraction uses the source produced by the preceding edit.

```tspp main.tspp
const first = 1 + 2;
              ^^^^^ selection:first
const second = 3 + 4;
               ^^^^^ selection:second
```

```query extract_variable main.tspp#selection:first new_name=firstValue apply
```

```tspp main.tspp after
const firstValue = 1 + 2;
const first = firstValue;
const second = 3 + 4;
               ^^^^^ selection:second
```

```query extract_variable main.tspp#selection:second new_name=secondValue
```

```tspp main.tspp after
const firstValue = 1 + 2;
const first = firstValue;
const secondValue = 3 + 4;
const second = secondValue;
```

## Module Expression

### Extract a module initializer

A module expression becomes a preceding module binding.

```tspp main.tspp
const value = 1 + 2;
              ^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=computed
```

```tspp main.tspp after
const computed = 1 + 2;
const value = computed;
```

## No Edit

### Reject an invalid binding name

An invalid identifier produces no edit.

```tspp main.tspp
const value = 1 + 2;
              ^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=bad-name
@extract_variable.none
```

### Reject an existing identifier

Extracting an identifier under the same name produces no edit.

```tspp main.tspp
const extracted = 1;
const value = extracted;
              ^^^^^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=extracted
@extract_variable.none
```

### Reject a partial expression

A selection that does not cover one complete expression produces no edit.

```tspp main.tspp
const value = 10 + 20;
              ^^^^ selection
```

```query extract_variable main.tspp#selection new_name=part
@extract_variable.none
```

## Call Arguments

### Extract a call argument

The new binding stays in the call's statement scope.

```tspp main.tspp
function consume(value: int32): void {}

function main(left: int32, right: int32): void {
    consume(left + right);
            ^^^^^^^^^^^^ selection
}
```

```query extract_variable main.tspp#selection new_name=total
```

```tspp main.tspp after
function consume(value: int32): void {}

function main(left: int32, right: int32): void {
    const total = left + right;
    consume(total);
}
```

### Preserve argument evaluation order

Extraction is unavailable when moving an argument would cross an earlier call.

```tspp main.tspp
function next(): int32 {
    return 1;
}

function consume(left: int32, right: int32): void {}

consume(next(), 1 + 2);
                ^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=total
@extract_variable.none
```

## Object Literals

### Extract an object property value

Extraction preserves the surrounding property.

```tspp main.tspp
const configuration = { total: 1 + 2 };
                               ^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=computed
```

```tspp main.tspp after
const computed = 1 + 2;
const configuration = { total: computed };
```

## Member Access

### Extract a member access

The new binding remains in the owning local scope.

```tspp main.tspp
struct User {
    name: string;
}

function label(user: User): string {
    const value = user.name;
                  ^^^^^^^^^ selection
    return value;
}
```

```query extract_variable main.tspp#selection new_name=name
```

```tspp main.tspp after
struct User {
    name: string;
}

function label(user: User): string {
    const name = user.name;
    const value = name;
    return value;
}
```

## Initializers

### Extract a local initializer

Extraction inserts the binding immediately before its declaration.

```tspp main.tspp
function main(): void {
    const value = 1 + 2;
                  ^^^^^ selection
}
```

```query extract_variable main.tspp#selection new_name=computed
```

```tspp main.tspp after
function main(): void {
    const computed = 1 + 2;
    const value = computed;
}
```

## Control Flow

### Extract inside a branch

The new binding remains inside the branch that controls its evaluation.

```tspp main.tspp
function choose(enabled: boolean): int32 {
    if (enabled) {
        return 1 + 2;
               ^^^^^ selection
    }

    return 0;
}
```

```query extract_variable main.tspp#selection new_name=selected
```

```tspp main.tspp after
function choose(enabled: boolean): int32 {
    if (enabled) {
        const selected = 1 + 2;
        return selected;
    }

    return 0;
}
```

## Declarators

### Extract a later declarator initializer

Splitting the declaration preserves initializer order.

```tspp main.tspp
const first = 1, second = 2 + 3;
                          ^^^^^ selection
```

```query extract_variable main.tspp#selection new_name=sum
```

```tspp main.tspp after
const first = 1;
const sum = 2 + 3;
const second = sum;
```
