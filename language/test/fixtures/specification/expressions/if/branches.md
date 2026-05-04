# If Let

## syntax

### if let returns union from branches

> If let yields the union of branch types when else is present.

```ds
declare const value: int32;

const result = if let 1 = value {
    "one"
} else {
    2
};

result satisfies string | int32;
```

### if let can appear in else if chains

> If let conditions can appear in else if branches.

```ds
declare const value: 1 | 2 | 3;

const result = if let 1 = value {
    "one"
} else if let 2 = value {
    "two"
} else {
    "three"
};

result satisfies string;
```

### if let without else yields void

> If let without else yields void.

```ds
declare const value: int32;

const result: int32 = if let 1 = value {
    1
};
```

- contains: not assignable

## Scoping

### if let bindings are scoped to the then branch

> Bindings introduced by if let are only visible in the then branch.

```ds
declare const pair: (int32, int32);

const result = if let (left, right) = pair {
    left + right
} else {
    left
};
```

- contains: missing symbol

### if let bindings do not escape the if expression

> Bindings introduced by if let are not visible after the if.

```ds
declare const value: int32;

if let x = value {
    x
}

x
```

- contains: missing symbol

## Flow

### if let narrows values in both branches

> If let narrows the matched value in the then and else branches.

```ds
declare const value: 1 | 2;

if let 1 = value {
    value satisfies 1;
} else {
    value satisfies 2;
}
```

## Annotations

### if let type annotations accept compatible values

> Type annotations on if let bindings accept compatible values.

```ds
declare const value: int32;

if let x: int32 = value {
    x satisfies int32;
}
```

### if let type annotations constrain the value

> Type annotations on if let bindings must be satisfied by the matched value.

```ds
declare const value: string | int32;

if let x: int32 = value {
    x
}
```

- expected int32, found string | int32 (not assignable)
