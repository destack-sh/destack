# Match Narrowing

Match arms narrow the scrutinee based on the matched pattern.
Match expression results still uses best common type rules.

## Discriminants

### match narrows discriminated unions per arm

> Patterns narrows discriminated unions inside each arm.

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

## Literal unions

### match narrows literal unions per arm

> Literal patterns narrow the matched value inside each arm.

```ds
type Status = "ready" | "loading";

declare let status: Status;

match (status) {
    "ready" => {
        status satisfies "ready";
    }
    "loading" => {
        status satisfies "loading";
    }
}
```

### match narrows union literal patterns per arm

> Union literal patterns narrow the scrutinee to the covered subset.

```ds
type Status = 1 | 2 | 3;

declare let status: Status;

match (status) {
    1 | 2 => {
        status satisfies 1 | 2;
    }
    3 => {
        status satisfies 3;
    }
}
```

### match narrows tuple unions by literal discriminant positions

> Tuple discriminants narrow tuple unions to the matched branch.

```ds
type Pair = (1, string) | (2, string);

declare let pair: Pair;

match (pair) {
    (1, value) => {
        value satisfies string;
    }
    (2, value) => {
        value satisfies string;
    }
}
```

### match narrows heterogeneous tuple payloads by discriminant

> Tuple discriminant branches narrow the payload slot to the matching member type.

```ds
type Pair = (1, string) | (2, int32);

declare let pair: Pair;

match (pair) {
    (1, value) => {
        value satisfies string;
    }
    (2, value) => {
        value satisfies int32;
    }
}
```

### match narrows tuple members that contain discriminated objects

> Tuple discriminants can narrow nested discriminated object payloads.

```ds
type Event =
    | (1, { kind: "text", value: string })
    | (2, { kind: "code", value: int32 });

declare let event: Event;

match (event) {
    (1, { kind: "text", value }) => {
        value satisfies string;
    }
    (2, { kind: "code", value }) => {
        value satisfies int32;
    }
}
```

### match narrows discriminated object members that contain tuples

> Tuple discriminants in object payloads can narrow tuple member types.

```ds
type Envelope =
    | { kind: "text", payload: (1, string) }
    | { kind: "code", payload: (2, int32) };

declare let envelope: Envelope;

match (envelope) {
    { payload: (1, value) } => {
        value satisfies string;
    }
    { payload: (2, value) } => {
        value satisfies int32;
    }
}
```

### match object wildcard filters preserve non-discriminant unions

> Wildcard object field filters does not over-narrow unrelated payload members.

```ds
type Envelope =
    | { kind: "text", payload: string }
    | { kind: "code", payload: int32 };

declare let envelope: Envelope;

match (envelope) {
    { kind: _, payload } => {
        payload satisfies string | int32;
    }
}
```

### match narrows nested discriminant fields recursively

> Nested object discriminants narrow payload fields recursively.

```ds
type Envelope =
    | { kind: "text", data: { tag: 1, value: string } }
    | { kind: "code", data: { tag: 2, value: int32 } };

declare let envelope: Envelope;

match (envelope) {
    { data: { tag: 1, value } } => {
        value satisfies string;
    }
    { data: { tag: 2, value } } => {
        value satisfies int32;
    }
}
```

## Match results

### match results use best common type

> Match expressions use best common type for their result.

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
