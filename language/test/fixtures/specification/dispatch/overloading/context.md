# Overload Contextual Typing

Overload selection should include contextual typing against each candidate.
When multiple candidates apply, declaration order should still win.

## Lambdas

### contextual typing follows overload declaration order

> Contextual lambda typing should be evaluated per overload candidate in order.

```ds
function apply(transform: (value: number) => number): "number" {
    return "number";
}

function apply(transform: (value: string) => string): "string" {
    return "string";
}

const selected = apply((value) => value);
selected satisfies "number";
```

### contextual typing does not select later overloads

> Later overloads should not win when earlier overloads are applicable.

```ds
function apply(transform: (value: number) => number): "number" {
    return "number";
}

function apply(transform: (value: string) => string): "string" {
    return "string";
}

const selected = apply((value) => value);
selected satisfies "string";
```

- contains: not assignable

## Optional and rest parameters

### optional parameters do not override earlier overloads

> Earlier overloads should win even when later overloads are compatible via optional parameters.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(value: number, other?: number): "two" {
    return "two";
}

const selected = pick(1);
selected satisfies "one";
```

### optional parameters do not select later overloads

> Later optional overloads should not win when earlier overloads apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(value: number, other?: number): "two" {
    return "two";
}

const selected = pick(1);
selected satisfies "two";
```

- contains: not assignable

### rest parameters do not override earlier overloads

> Rest parameters should not override earlier overloads when both can apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(...values: number[]): "many" {
    return "many";
}

const selected = pick(1);
selected satisfies "one";
```

### rest parameters do not select later overloads

> Later rest overloads should not win when earlier overloads apply.

```ds
function pick(value: number): "one" {
    return "one";
}

function pick(...values: number[]): "many" {
    return "many";
}

const selected = pick(1);
selected satisfies "many";
```

- contains: not assignable
