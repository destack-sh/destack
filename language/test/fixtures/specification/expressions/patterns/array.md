# Array Patterns

## array patterns

### array patterns bind elements

> Array patterns bind elements by position.

```ds
let [first, second] = [1, 2];
first satisfies int32;
second satisfies int32;
```

### array destructuring requires an initializer

> Destructuring declarations require an initializer.

```ts
let [value]: number[];
```

- contains: destructuring declarations require initializers

### array patterns cannot use named fields in TypeScript

> Named fields are not allowed in array patterns in TypeScript.

```ts:main.ts
let [x: y] = [1, 2];
```

- contains: named fields are not allowed in array or tuple patterns

### array patterns bind readonly named identifiers

> `readonly` remains an identifier in array destructuring patterns.

```ts
const [readonly, setReadonly] = [1, 2];
readonly satisfies number;
setReadonly satisfies number;
```
