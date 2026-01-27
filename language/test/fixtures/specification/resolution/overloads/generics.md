# Generic Overload Order

Generic overloads can shadow more specific overloads when they appear first.
Destack uses declaration order, so ordering must be explicit and stable.

## Shadowing

### generic overloads can shadow specific overloads

> A generic overload declared first should win even for specific inputs.

```ds
function classify<T>(value: T): "generic" {
    return "generic";
}

function classify(value: "x"): "specific" {
    return "specific";
}

const selected = classify("x");
selected satisfies "generic";
```

### generic overloads do not select later specific overloads

> Later overloads should not win when a generic overload is first and applicable.

```ds
function classify<T>(value: T): "generic" {
    return "generic";
}

function classify(value: "x"): "specific" {
    return "specific";
}

const selected = classify("x");
selected satisfies "specific";
```

- contains: not assignable

### specific overloads win when declared first

> A specific overload declared first should win for matching inputs.

```ds
function classify(value: "x"): "specific" {
    return "specific";
}

function classify<T>(value: T): "generic" {
    return "generic";
}

const selected = classify("x");
selected satisfies "specific";
```

### specific overloads do not select later generic overloads

> Later generic overloads should not win when a specific overload is first.

```ds
function classify(value: "x"): "specific" {
    return "specific";
}

function classify<T>(value: T): "generic" {
    return "generic";
}

const selected = classify("x");
selected satisfies "generic";
```

- contains: not assignable
