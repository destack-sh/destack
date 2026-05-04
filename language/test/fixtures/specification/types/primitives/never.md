# Never Type

The `never` type (bottom type).

## never is assignable to everything

### never to number

> Never is the bottom type, assignable to any type.

```ds
function fail(): never { throw "error" }
const x: number = fail()
```

### never to string

> A function returning never can be assigned to any type.

```ds
function fail(): never { throw "error" }
const x: string = fail()
```

## values are not assignable to never

### number to never is rejected

> Concrete values are not assignable to never.

```ds
const x: never = 1;
```

- contains: not assignable

### string to never is rejected

> String values are not assignable to never.

```ds
const x: never = "no";
```

- contains: not assignable

### union with never simplifies to the other member

> Unions with never simplifies to the non-never member.

```ds
type Value = never | string;

const x: Value = "ok";
x satisfies string;
```

### intersection with never rejects all concrete values

> Intersections with never collapse to never and reject concrete values.

```ds
type Value = never & string;

const x: Value = "ok";
```

- contains: not assignable
