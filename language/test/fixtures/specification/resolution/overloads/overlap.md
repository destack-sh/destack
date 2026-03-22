# Overload Overlap and Order

Overloads can overlap structurally, and Destack resolves them by declaration order.
This must be explicit so callers can control which overload wins.

## Overlap

### broad overloads shadow narrow overloads when first

> A broad overload declared first should shadow narrower overloads.

```ds
function pick(value: number): "broad" {
    return "broad";
}

function pick(value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick(1);
selected satisfies "broad";
```

### broad overloads do not select later narrow overloads

> Later narrow overloads should not win when a broad overload is first and applicable.

```ds
function pick(value: number): "broad" {
    return "broad";
}

function pick(value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick(1);
selected satisfies "narrow";
```

- not assignable

### narrow overloads win when declared before broad overloads

> A narrow overload declared first should win for matching literals.

```ds
function pick(value: 1 | 2): "narrow" {
    return "narrow";
}

function pick(value: number): "broad" {
    return "broad";
}

const selected = pick(1);
selected satisfies "narrow";
```

### narrow-first declarations do not select later broad overloads

> Later broad overloads should not replace earlier narrow matches.

```ds
function pick(value: 1 | 2): "narrow" {
    return "narrow";
}

function pick(value: number): "broad" {
    return "broad";
}

const selected = pick(1);
selected satisfies "broad";
```

- not assignable

### generic-first overlap shadows literal overloads

> Generic overlap declared first should shadow later literal overloads.

```ds
function classify<T>(value: T): "generic" {
    return "generic";
}

function classify(value: "x"): "literal" {
    return "literal";
}

const selected = classify("x");
selected satisfies "generic";
```
