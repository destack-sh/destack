# Call/Apply/Bind

Tests for strict `bind`, `call`, and `apply` checking.

## strictBindCallApply

### strict call checks this arguments

> `call` enforces the target `this` type when strict checking is enabled.

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

- contains: not assignable

### strict apply checks argument tuples

> `apply` validates tuple arguments against the function signature.

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.apply({ base: 1 }, ["no"]);
```

- contains: not assignable

### strict bind preserves parameter types

> `bind` returns a function with the original parameter types.

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "strictBindCallApply": true } }
```

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

const bound = add.bind({ base: 1 });
const result = bound(2);
result satisfies number;
```
