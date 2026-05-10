# Match Narrowing

Match arms narrow the matched value from the selected pattern.
Result typing is still handled by arm body joins.

## discriminants

### discriminants narrow each arm

Object discriminants select the matching union member.

```ds
type Shape = { kind: "circle"; radius: number } | { kind: "square"; size: number };

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

## literal unions

### literals narrow each arm

Literal patterns select the matching literal member.

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

### union literal patterns narrow covered values

Union patterns select the covered literal members.

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

### tuple discriminants narrow unions

Literal tuple positions select the matching tuple member.

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

### tuple discriminants narrow payloads

Tuple payload slots narrow with the selected tuple member.

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

### tuple discriminants narrow nested objects

Nested object payloads narrow with the selected tuple member.

```ds
type Event = (1, { kind: "text"; value: string }) | (2, { kind: "code"; value: int32 });

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

### object payloads narrow nested tuples

Nested tuple payloads narrow from literal tuple positions.

```ds
type Envelope = { kind: "text"; payload: (1, string) } | { kind: "code"; payload: (2, int32) };

declare let envelope: Envelope;

match (envelope) {
    {
        payload: (1, value),
    } => {
        value satisfies string;
    }
    {
        payload: (2, value),
    } => {
        value satisfies int32;
    }
}
```

### wildcard fields leave payload unions intact

Ignored discriminant fields do not narrow unrelated payloads.

```ds
type Envelope = { kind: "text"; payload: string } | { kind: "code"; payload: int32 };

declare let envelope: Envelope;

match (envelope) {
    { kind: _, payload } => {
        payload satisfies string | int32;
    }
}
```

### nested discriminants narrow payloads

Nested object tags select the matching payload member.

```ds
type Envelope =
    | { kind: "text"; data: { tag: 1; value: string } }
    | { kind: "code"; data: { tag: 2; value: int32 } };

declare let envelope: Envelope;

match (envelope) {
    {
        data: { tag: 1, value },
    } => {
        value satisfies string;
    }
    {
        data: { tag: 2, value },
    } => {
        value satisfies int32;
    }
}
```

## match results

### match results join arm bodies

Arm body types join into the match result.

```ds
type Shape = { kind: "circle"; radius: number } | { kind: "square"; size: number };

declare let shape: Shape;

const area = match (shape) {
    { kind: "circle", radius } => radius
    { kind: "square", size } => size
};

area satisfies number;
```
