# Overloads With This Parameters

Overloads that differ by `this` parameters should still follow declaration order.
Strict bind, call, and apply checking should apply to `this` parameter overloads.

## strictBindCallApply

### this parameter overloads select the first overload

> The first matching `this` parameter overload should win.

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "strictBindCallApply": true } }
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

```json:dsconfig.json
{ "compilerOptions": { "strictBindCallApply": true } }
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
