# Never Type

Tests for the `never` type (bottom type).

## Never is Assignable to Everything

### never to number

> Never is the bottom type, assignable to any type.

```ds
function fail(): never { throw new Error() }
const x: number = fail()
```

### never to string

> A function returning never can be assigned to any type.

```ds
function fail(): never { throw new Error() }
const x: string = fail()
```
