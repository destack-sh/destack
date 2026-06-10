# Range Patterns

Range patterns test integer, `bigint`, and `char` intervals.

## bounds

### half-open range pattern

A matched range narrows the value to the interval type.

```ds
function label(value: int32): string {
    match (value) {
        0..10 => {
            value satisfies 0..10;
            "small"
        }
        _ => {
            value satisfies int32;
            "other"
        }
    }
}
```

### inclusive range pattern

`..=` patterns include the end.

```ds
function isByte(value: int32): boolean {
    match (value) {
        0..=255 => {
            value satisfies 0..=255;
            true
        }
        _ => false
    }
}
```

### one-sided range patterns

Open ends cover everything below or above.

```ds
function sign(value: int32): string {
    match (value) {
        ..0 => {
            value satisfies ..0;
            "negative"
        }
        0 => "zero"
        1.. => {
            value satisfies 1..;
            "positive"
        }
    }
}
```

## alternatives

### range alternatives

Ranges combine with `|` like any other pattern.

```ds
function isEdge(value: int32): boolean {
    match (value) {
        0..10 | 90..=99 => {
            value satisfies 0..10 | 90..=99;
            true
        }
        _ => false
    }
}
```

### constant bounds

Bounds can be named constants.

```ds
const LOW: int32 = 10;
const HIGH: int32 = 20;

function isMiddle(value: int32): boolean {
    match (value) {
        LOW..HIGH => {
            value satisfies LOW..HIGH;
            true
        }
        _ => false
    }
}
```

### char range patterns

`char` intervals match scalar values.

```ds
function kind(value: char): string {
    match (value) {
        'a'..='z' => {
            value satisfies 'a'..='z';
            "lower"
        }
        'A'..='Z' => {
            value satisfies 'A'..='Z';
            "upper"
        }
        _ => "other"
    }
}
```

## invalid forms

### float range patterns are rejected

Floats are not a bounded set, so they do not form intervals.

```ds
function classify(value: float64): string {
    match (value) {
        0.0..1.0 => "unit"
        _ => "other"
    }
}
```

- contains: range pattern

### float checks use guards

A guard expresses the same float check explicitly.

```ds
function classify(value: float64): string {
    match (value) {
        value if (0.0 <= value && value < 1.0) => "unit"
        _ => "other"
    }
}
```
