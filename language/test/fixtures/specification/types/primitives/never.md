# Never Type

The `never` type (bottom type).

## Never is Assignable to Everything

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

## Values Are Not Assignable to Never

### number to never is rejected

> Concrete values are not assignable to never.

```ds
const x: never = 1;
```

- type "ok" is not assignable to type Value

### string to never is rejected

> String values are not assignable to never.

```ds
const x: never = "no";
```

- type "ok" is not assignable to type Value

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

- type "ok" is not assignable to type Value
