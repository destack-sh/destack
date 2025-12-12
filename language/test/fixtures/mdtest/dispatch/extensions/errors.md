# Extension Errors

Tests for error cases with extensions.

## Missing Methods

### calling nonexistent extension method

> Calling a method that doesn't exist on the type or extensions is an error.

```ds
struct Point { x: number, y: number }

extension Point {
    magnitude(): number { return 0 }
}

declare function getPoint(): Point;

const p = getPoint();
const bad = p.nonexistent();
```

- contains: does not exist

### method exists on type but not extension

> Methods defined on the type itself are accessible without extension.

```ds
struct Point {
    x: number,
    y: number
}

declare function getPoint(): Point;

const p = getPoint();
const x: number = p.x;
```

## Return Type Errors

### wrong return type annotation

> Return type of extension method is checked.

```ds
struct Foo {}

extension Foo {
    bar(): number { return 0 }
}

declare function getFoo(): Foo;

const f = getFoo();
const s: string = f.bar();
```

- contains: not assignable

## Multiple Extensions

### shadowing between extensions

> When multiple extensions define the same method, the first one wins.

```ds
struct Data {}

extension Data {
    process(): number { return 1 }
}

extension Data {
    process(): string { return "" }
}

declare function getData(): Data;

const d = getData();
const result: number = d.process();
```
