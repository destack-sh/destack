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

### array patterns need iterable values

> Array destructuring patterns require iterable initializers.

```ds
let [value] = 1;
```

- contains: not iterable

## defaults

### array defaults fill absent elements

> Defaults bind when the matched element is absent.

```ds
let [value = 1] = [];
value satisfies int32;
```

### array defaults check element types

> Defaults must satisfy the declared element type.

```ds
let [value = "no"]: int32[] = [];
```

- contains: not assignable

## rest

### array rest binds tails

> Rest patterns collect trailing elements.

```ds
let [head, ...tail] = [1, 2, 3];
head satisfies int32;
tail satisfies int32[];
```

### slice rest keeps slices

> Slice rest patterns bind the tail as a slice.

```ds
declare const values: [int32];

let [head, ...tail] = values;
head satisfies int32;
tail satisfies [int32];
```

### array rest is last

> Rest patterns cannot be followed by more elements.

```ds
let [...middle, last] = [1, 2, 3];
```

- contains: rest
