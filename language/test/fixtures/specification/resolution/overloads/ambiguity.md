# Overload Ambiguity Policy

Destack resolves overloads by declaration order.
When multiple overloads match, the first matching overload should win.
Ambiguous overload errors should be reserved for cases with no declared order winner.

## Declaration order

### any inputs follow declaration order

> An `any` argument should resolve to the first applicable overload.

```ds
function parse(value: string): "string" {
    return "string";
}

function parse(value: number): "number" {
    return "number";
}

declare let value: any;

const selected = parse(value);
selected satisfies "string";
```

```json:destack.json
{ "compilerOptions": { "noAny": false } }
```

### any inputs do not select later overloads

> Later overloads should not win for `any` arguments when earlier overloads apply.

```ds
function parse(value: string): "string" {
    return "string";
}

function parse(value: number): "number" {
    return "number";
}

declare let value: any;

const selected = parse(value);
selected satisfies "number";
```

- contains: not assignable

```json:destack.json
{ "compilerOptions": { "noAny": false } }
```

### never inputs follow declaration order

> A `never` argument should resolve to the first applicable overload.

```ds
function parse(value: string): "string" {
    return "string";
}

function parse(value: number): "number" {
    return "number";
}

declare let value: never;

const selected = parse(value);
selected satisfies "string";
```

### never inputs do not select later overloads

> Later overloads should not win for `never` arguments when earlier overloads apply.

```ds
function parse(value: string): "string" {
    return "string";
}

function parse(value: number): "number" {
    return "number";
}

declare let value: never;

const selected = parse(value);
selected satisfies "number";
```

- contains: not assignable

### unknown inputs follow declaration order

> An `unknown` argument should resolve to the first applicable overload.

```ds
function parse(value: any): "any" {
    return "any";
}

function parse(value: unknown): "unknown" {
    return "unknown";
}

declare let value: unknown;

const selected = parse(value);
selected satisfies "any";
```

```json:destack.json
{ "compilerOptions": { "noAny": false } }
```

### unknown inputs do not select later overloads

> Later overloads should not win for `unknown` arguments when earlier overloads apply.

```ds
function parse(value: any): "any" {
    return "any";
}

function parse(value: unknown): "unknown" {
    return "unknown";
}

declare let value: unknown;

const selected = parse(value);
selected satisfies "unknown";
```

- contains: not assignable

```json:destack.json
{ "compilerOptions": { "noAny": false } }
```
