# If Let

## branches

### if (let ...) joins branches

If let joins then and else branch types.

```ds
declare const value: int32;

const result = if (let 1 = value) {
    "one"
} else {
    2
};

result satisfies string | int32;
```

### if (let ...) chains with else if

If let conditions are valid in else if branches.

```ds
declare const value: 1 | 2 | 3;

const result = if (let 1 = value) {
    "one"
} else if (let 2 = value) {
    "two"
} else {
    "three"
};

result satisfies string;
```

### if (let ...) without else yields void

If let without else yields void.

```ds
declare const value: int32;

const result: int32 = if (let 1 = value) {
    1
};
```

- contains: not assignable

## scoping

### if (let ...) bindings stay in the then branch

Bindings introduced by if (let ...) are only visible in the then branch.

```ds
declare const pair: (int32, int32);

const result = if (let (left, right) = pair) {
    left + right
} else {
    left
};
```

- contains: missing symbol

### if (let ...) bindings do not escape the if expression

Bindings introduced by if (let ...) are not visible after the if.

```ds
declare const value: int32;

if (let x = value) {
    x
}

x
```

- contains: missing symbol

## flow

### if (let ...) narrows both branches

If let narrows the matched value in the then and else branches.

```ds
declare const value: 1 | 2;

if (let 1 = value) {
    value satisfies 1;
} else {
    value satisfies 2;
}
```

## annotations

### if (let ...) annotations accept matching values

Type annotations on if (let ...) bindings check the matched value.

```ds
declare const value: int32;

if (let x: int32 = value) {
    x satisfies int32;
}
```

### if (let ...) annotations reject mismatches

The matched value must satisfy the binding annotation.

```ds
declare const value: string | int32;

if (let x: int32 = value) {
    x
}
```

- contains: not assignable
