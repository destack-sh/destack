# Equality

`==` and `!=` use value equality, while `===` and `!==` are restricted to identity-compatible values.

## loose equality

### number equals number

> Numeric equality produces boolean.

```ds
const value = 1 == 1;
value satisfies boolean;
```

### string equals string

> String equality produces boolean.

```ds
const value = "a" == "b";
value satisfies boolean;
```

### not equal produces boolean

> `!=` produces boolean.

```ds
const value = 1 != 2;
value satisfies boolean;
```

## strict equality

### strict equality accepts primitives

> `===` and `!==` are valid on primitive identity-compatible values.

```ds
const same = 1 === 1;
same satisfies boolean;

const different = 1 !== 2;
different satisfies boolean;
```

### strict equality rejects structs

> Struct values do not have identity equality.

```ds
struct Point { x: int }

declare function getPoint(): Point;

const left = getPoint();
const right = getPoint();

left === right;
left !== right;
```

- contains: strict equality

### strict equality allows classes

> Class instances have identity equality.

```ds
class Box {}

declare const left: Box;
declare const right: Box;

const same = left === right;
same satisfies boolean;
```

## overloads

### equality dispatches to Equal

> `==` and `!=` dispatch to `Equal` on the receiver.

```ds
struct Measure { value: int }

extension of Measure implements Equal<Measure> {
    equal(other: Measure): boolean { return true }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

const isEqual = left == right;
isEqual satisfies boolean;

const isNotEqual = left != right;
isNotEqual satisfies boolean;
```

### equality requires Equal

> Value equality requires `Equal` for non-builtin value types.

```ds
struct Measure { value: int }

extension of Measure implements Compare<Measure> {
    compare(other: Measure): Ordering { return Ordering.Equal }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

left == right;
```

- contains: no matching overload
