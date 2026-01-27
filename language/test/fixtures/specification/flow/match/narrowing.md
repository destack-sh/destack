# Match Narrowing

Match arms should narrow the scrutinee based on the matched pattern.
Match expression results should still use best common type rules.

## Discriminants

### match narrows discriminated unions per arm

> Patterns should narrow discriminated unions inside each arm.

```ds
type Shape =
    | { kind: "circle", radius: number }
    | { kind: "square", size: number };

declare let shape: Shape;

match (shape) {
    { kind: "circle", radius } => {
        radius satisfies number;
        shape.kind satisfies "circle";
    }
    { kind: "square", size } => {
        size satisfies number;
        shape.kind satisfies "square";
    }
}
```

## Match results

### match results use best common type

> Match expressions should use best common type for their result.

```ds
type Shape =
    | { kind: "circle", radius: number }
    | { kind: "square", size: number };

declare let shape: Shape;

const area = match (shape) {
    { kind: "circle", radius } => radius
    { kind: "square", size } => size
};

area satisfies number;
```

