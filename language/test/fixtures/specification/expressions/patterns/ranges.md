# Range Patterns

Range patterns test integer, `bigint`, and `char` intervals.

## bounds

### half-open range pattern

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

```ds
function classify(value: float64): string {
    match (value) {
        value if (0.0 <= value && value < 1.0) => "unit"
        _ => "other"
    }
}
```
