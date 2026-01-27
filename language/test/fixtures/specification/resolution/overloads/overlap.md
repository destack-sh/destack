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

- contains: not assignable
