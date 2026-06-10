# Logical Narrowing

Logical operators combine and invert guard narrowing (there is no truthiness, so the operands are real guards).

## and guards

### and guard narrows to non nullish

Both conjuncts apply in the then branch, neither is certain in the else.

```ds
const value: string | null | undefined = null;
if (value !== null && value !== undefined) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### and guard narrows mixed nullish symbols

Each conjunct narrows its own symbol.

```ds
const left: string | null = null;
const right: string | undefined = undefined;
if (left !== null && right !== undefined) {
    left satisfies string;
    right satisfies string;
} else {
    left satisfies string | null;
    right satisfies string | undefined;
}
```

### and guard narrows the right side

The right operand sees the left conjunct's narrowing.

```ds
const value: string | null = null;
const acceptsString = (input: string): boolean => true;
if (value !== null && acceptsString(value)) {
    value satisfies string;
}
```

### and guard narrows with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null !== value && undefined !== value) {
    value satisfies string;
} else {
    value satisfies string | null | undefined;
}
```

### and guard narrows with parentheses

Grouping does not change the narrowing.

```ds
const value: string | null | undefined = null;
if (value !== null && value !== undefined) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

## or guards

### or guard narrows to nullish

A disjunction narrows its else branch.

```ds
const value: string | null | undefined = null;
if (value === null || value === undefined) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### or guard narrows both symbols in the else branch

The else branch knows every disjunct failed.

```ds
const left: string | null = null;
const right: string | null = null;
if (left === null || right === null) {
    left satisfies string | null;
    right satisfies string | null;
} else {
    left satisfies string;
    right satisfies string;
}
```

### or guard narrows the right side

The right operand sees the left disjunct's failure.

```ds
const value: string | null = null;
const acceptsString = (input: string): boolean => true;
if (value === null || acceptsString(value)) {
    value satisfies string | null;
} else {
    value satisfies string;
}
```

### or guard narrows with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null === value || undefined === value) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### or guard narrows chained symbols in the else branch

Chained disjuncts all fail in the else branch.

```ds
const left: string | null = null;
const middle: string | null = null;
const right: string | null = null;
if (left === null || middle === null || right === null) {
    left satisfies string | null;
    middle satisfies string | null;
    right satisfies string | null;
} else {
    left satisfies string;
    middle satisfies string;
    right satisfies string;
}
```

## not guards

### not guard inverts narrowing

`!` swaps the branches.

```ds
const value: string | null | undefined = null;
if (!(value === null)) {
    value satisfies string | undefined;
} else {
    value satisfies null;
}
```

### not guard narrows an and expression

De Morgan applies to conjunctions.

```ds
const left: string | null = null;
const right: string | null = null;
if (!(left !== null && right !== null)) {
    left satisfies string | null;
    right satisfies string | null;
} else {
    left satisfies string;
    right satisfies string;
}
```

### not guard narrows an or expression

De Morgan applies to disjunctions.

```ds
const left: string | null = null;
const right: string | null = null;
if (!(left === null || right === null)) {
    left satisfies string;
    right satisfies string;
} else {
    left satisfies string | null;
    right satisfies string | null;
}
```

### not guard narrows a non strict nullish guard

`!= null` covers both nullish values, inverted.

```ds
const value: string | null | undefined = null;
if (!(value != null)) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

## multi symbol guards

### and guard narrows multiple symbols

Each conjunct narrows only what it tests.

```ds
const left: string | null | undefined = null;
const right: string | null | undefined = null;
if (left !== null && right !== null) {
    left satisfies string | undefined;
    right satisfies string | undefined;
} else {
    left satisfies string | null | undefined;
    right satisfies string | null | undefined;
}
```

### and guard narrows chained symbols

Long chains accumulate every conjunct.

```ds
const left: string | null = null;
const middle: string | undefined = undefined;
const right: string | null = null;
if (left !== null && middle !== undefined && right !== null) {
    left satisfies string;
    middle satisfies string;
    right satisfies string;
} else {
    left satisfies string | null;
    middle satisfies string | undefined;
    right satisfies string | null;
}
```
