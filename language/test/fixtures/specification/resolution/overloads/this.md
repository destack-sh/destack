# Overloads With This Parameters

Overloads that differ by `this` parameters should still follow declaration order.
Strict bind, call, and apply checking should apply to `this` parameter overloads.

## strictBindCallApply

### this parameter overloads select the first overload

> The first matching `this` parameter overload should win.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

function use(this: { kind: "a" }, value: number): "narrow" {
    return "narrow";
}

const selected = use.call({ kind: "a" }, 1);
selected satisfies "broad";
```

### this parameter overloads do not select later overloads

> Later `this` parameter overload results should not be selected.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

function use(this: { kind: "a" }, value: number): "narrow" {
    return "narrow";
}

const selected = use.call({ kind: "a" }, 1);
selected satisfies "narrow";
```

- contains: not assignable

### this parameter overloads apply through bind with declaration order

> Bound calls should preserve declaration-order overload selection for `this` parameters.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

function use(this: { kind: "a" }, value: number): "narrow" {
    return "narrow";
}

const bound = use.bind({ kind: "a" });
const selected = bound(1);
selected satisfies "broad";
```

### strictBindCallApply rejects incompatible this arguments

> Strict bind/call/apply checks should reject incompatible `this` arguments.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

use.call({ kind: 1 }, 1);
```

- contains: not assignable

### this parameter overloads preserve declaration order for apply

> `.apply` should preserve declaration-order selection for `this` overloads.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

function use(this: { kind: "a" }, value: number): "narrow" {
    return "narrow";
}

const selected = use.apply({ kind: "a" }, [1]);
selected satisfies "broad";
```

### this parameter overloads with apply do not select later overloads

> Later `this` overloads should not win through `.apply`.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function use(this: { kind: string }, value: number): "broad" {
    return "broad";
}

function use(this: { kind: "a" }, value: number): "narrow" {
    return "narrow";
}

const selected = use.apply({ kind: "a" }, [1]);
selected satisfies "narrow";
```

- contains: not assignable
