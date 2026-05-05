# Truthiness Narrowing

Boolean guard narrowing with logical operators.

## and guards

### and guard narrows to non nullish

```ds
const value: string | null | undefined = null;
if (value !== null && value !== undefined) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### and guard narrows mixed nullish symbols

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

```ds
const value: string | null = null;
const accepts_string = (input: string): boolean => true;
if (value !== null && accepts_string(value)) {
    value satisfies string;
}
```

### and guard narrows with symbol on right

```ds
const value: string | null | undefined = null;
if (null !== value && undefined !== value) {
    value satisfies string;
} else {
    value satisfies string | null | undefined;
}
```

### and guard narrows with parentheses

```ds
const value: string | null | undefined = null;
if ((value !== null) && (value !== undefined)) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

## or guards

### or guard narrows to nullish

```ds
const value: string | null | undefined = null;
if (value === null || value === undefined) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### or guard narrows both symbols in the else branch

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

```ds
const value: string | null = null;
const accepts_string = (input: string): boolean => true;
if (value === null || accepts_string(value)) {
    value satisfies string | null;
} else {
    value satisfies string;
}
```

### or guard narrows with symbol on right

```ds
const value: string | null | undefined = null;
if (null === value || undefined === value) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### or guard narrows chained symbols in the else branch

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

```ds
const value: string | null | undefined = null;
if (!(value === null)) {
    value satisfies string | undefined;
} else {
    value satisfies null;
}
```

### not guard narrows an and expression

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
