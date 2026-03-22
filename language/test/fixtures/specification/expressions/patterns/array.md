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

- destructuring declarations require initializers

### array patterns cannot use named fields in TypeScript

> Named fields are not allowed in array patterns in TypeScript.

```ts:main.ts
let [x: y] = [1, 2];
```

- named fields are not allowed in array or tuple patterns

### array patterns bind readonly named identifiers

> `readonly` remains an identifier in array destructuring patterns.

```ts
const [readonly, setReadonly] = [1, 2];
readonly satisfies number;
setReadonly satisfies number;
```

### array patterns support rest bindings

> Array patterns support rest bindings for trailing elements.

```ds
let [head, ...tail] = [1, 2, 3];
head satisfies int32;
tail satisfies int32[];
```

### array patterns reject non iterable initializers

> Array destructuring patterns require iterable initializers.

```ds
let [value] = 1;
```

- not iterable
