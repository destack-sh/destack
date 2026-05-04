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

### array patterns cannot use named fields in `.ts` sources

> Named fields are not allowed in array patterns in `.ts` sources.

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

### array patterns reject non iterable initializers

> Array destructuring patterns require iterable initializers.

```ds
let [value] = 1;
```

- not iterable

## defaults

### array defaults fill missing elements

> Default values are used when the matched element is absent.

```ds
let [value = 1] = [];
value satisfies int32;
```

### array defaults must match declared element types

> Default values are checked against the binding pattern type.

```ds
let [value = "no"]: int32[] = [];
```

- contains: not assignable

## rest

### array rest binds trailing elements

> Rest patterns collect remaining elements.

```ds
let [head, ...tail] = [1, 2, 3];
head satisfies int32;
tail satisfies int32[];
```

### array rest must be last

> Rest patterns cannot be followed by more elements.

```ds
let [...middle, last] = [1, 2, 3];
```

- contains: rest
