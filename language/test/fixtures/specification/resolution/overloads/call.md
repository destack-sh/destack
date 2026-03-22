# Overloads With Call, Apply, And Bind

Call, apply, and bind should respect overload sets and declaration order.
Strict bind, call, and apply checking should still apply to overloads.

## strictBindCallApply

### call selects the first overload

> The first applicable overload should win through `.call`.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick.call(undefined, 1);
selected satisfies "broad";
```

### call does not select later overloads

> Later overloads should not win when an earlier overload is applicable.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick.call(undefined, 1);
selected satisfies "narrow";
```

- expected "narrow", found "broad" (not assignable)

### bind selects the first overload

> Bound functions should preserve overload order.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const bound = pick.bind(undefined);
const selected = bound(1);
selected satisfies "broad";
```

### bind does not select later overloads

> Later overloads should not win for bound calls.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const bound = pick.bind(undefined);
const selected = bound(1);
selected satisfies "narrow";
```

- expected "narrow", found "broad" (not assignable)

### apply selects the first overload

> `.apply` should preserve declaration-order overload selection.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick.apply(undefined, [1]);
selected satisfies "broad";
```

### apply does not select later overloads

> Later overloads should not win for `.apply`.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function pick(this: void, value: number): "broad" {
    return "broad";
}

function pick(this: void, value: 1 | 2): "narrow" {
    return "narrow";
}

const selected = pick.apply(undefined, [1]);
selected satisfies "narrow";
```

- expected "narrow", found "broad" (not assignable)