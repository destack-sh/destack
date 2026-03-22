# Call/Apply/Bind

Tests for strict `bind`, `call`, and `apply` checking.

## strictBindCallApply

### strict call checks this arguments

> `call` enforces the target `this` type when strict checking is enabled.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

- not assignable

### strict apply checks argument tuples

> `apply` validates tuple arguments against the function signature.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.apply({ base: 1 }, ["no"]);
```

- not assignable

### strict bind preserves parameter types

> `bind` returns a function with the original parameter types.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

const bound = add.bind({ base: 1 });
const result = bound(2);
result satisfies number;
```

### non-strict call allows mismatched this

> `call` skips strict checking when strictBindCallApply is false.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": false } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

### strict bind checks parameter assignments at call sites

> Bound call signatures preserve parameter type checking under strict checking.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

const bound = add.bind({ base: 1 });
bound("no");
```

- not assignable

### strict call checks argument arity

> Strict `call` checking enforces function argument arity.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: 1 });
```

- expected 2 arguments, found 1