# Interface Headers

## Interface Forms

### interface declaration

Extra whitespace in interface declarations should be normalized.

```ds
interface   Foo   {   }
```

Empty interface bodies stay on one line with internal spacing.

```ds expected
interface Foo {}
```

### interface with extends

Multiple extended interfaces are separated by comma and space.

```ds
interface   Foo   extends   Bar  ,  Baz   {   }
```

```ds expected
interface Foo extends Bar, Baz {}
```

### interface with property

Interfaces with members expand to multiple lines.
Properties use trailing semicolons.

```ds
interface Foo { x: number }
```

```ds expected
interface Foo {
    x: number;
}
```

### interface with multiple properties

Each property goes on its own line with a trailing semicolon.

```ds
interface Foo { x: number; y: string; z: boolean }
```

```ds expected
interface Foo {
    x: number;
    y: string;
    z: boolean;
}
```

### interface with method signature

Method signatures use declaration semicolons like other interface members.

```ds
interface Foo { bar(): void }
```

```ds expected
interface Foo {
    bar(): void;
}
```

### interface with static method signature

Static method signatures keep their modifier attached to the method.

```ds
interface Result { static fromError(error: E): this }
```

```ds expected
interface Result {
    static fromError(error: E): this;
}
```

### interface with method parameters

Method parameters follow standard parameter formatting rules.

```ds
interface Foo { add(a: number, b: number): number }
```

```ds expected
interface Foo {
    add(a: number, b: number): number;
}
```

## Export

### exported interface

The `export` keyword precedes the interface declaration.

```ds
export interface Foo { x: number }
```

```ds expected
export interface Foo {
    x: number;
}
```

## Line Breaking

### interface with many type params breaks

When type parameters exceed the line width, they break to multiple lines.

```ds line-width=40
interface Container<VeryLongType, AnotherType, ThirdType> { }
```

Each type parameter goes on its own line with a trailing comma.

```ds expected
interface Container<
    VeryLongType,
    AnotherType,
    ThirdType,
> {}
```

## Interface Members

### interface with mixed members

Properties and method signatures both use declaration semicolons.

```ds
interface User { id: number; name: string; email?: string; getName(): string; setName(name: string): void }
```

```ds expected
interface User {
    id: number;
    name: string;
    email?: string;
    getName(): string;
    setName(name: string): void;
}
```

## Documentation

### interface with doc comment

Doc comments are preserved above the interface declaration.

```ds
/// Represents a point in 2D space.
interface Point { x: number; y: number }
```

```ds expected
/// Represents a point in 2D space.
interface Point {
    x: number;
    y: number;
}
```
