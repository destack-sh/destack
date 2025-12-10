# Type Assertions (as)

Tests for type assertion (`as`) operator.

## Valid Casts

### same type

> Casting to the same type is always valid.

```ds
const x: number = 42;
const y = x as number;
y satisfies number;
```

### subtype to supertype

> Casting a subtype to a supertype is valid.

```ds
const x: 42 = 42;
const y = x as number;
y satisfies number;
```

### supertype to subtype

> Casting a supertype to a subtype is valid (type assertion).

```ds
declare const x: number;
const y = x as 42;
y satisfies 42;
```

### any to specific

> Casting from any to any type is valid.

```ds
declare const x: any;
const y = x as number;
y satisfies number;
```

### specific to any

> Casting any type to any is valid.

```ds
const x: number = 42;
const y = x as any;
y satisfies any;
```

### unknown to specific

> Casting from unknown to any type is valid.

```ds
declare const x: unknown;
const y = x as number;
y satisfies number;
```

## Invalid Casts

### unrelated types

> Casting between unrelated types is an error.

```ds
const x: string = "hello";
const y = x as number;
```

- cannot cast type string to number

### boolean to number

> Casting boolean to number is an error (no overlap).

```ds
const x: boolean = true;
const y = x as number;
```

- cannot cast type boolean to number

## Object Casts

### object widening

> Cast object to type with fewer required fields.

```ds
const obj = { x: 1, y: 2 };
const obj2 = obj as { x: number };
obj2 satisfies { x: number };
```

