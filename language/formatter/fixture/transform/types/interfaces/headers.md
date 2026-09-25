# Interface Headers

## Interface Forms

### interface declaration

Extra whitespace in interface declarations should be normalized.

```tspp
interface   Foo   {   }
```

Empty interface bodies stay on one line with internal spacing.

```tspp expected
interface Foo {}
```

### interface with extends

Multiple extended interfaces are separated by comma and space.

```tspp
interface   Foo   extends   Bar  ,  Baz   {   }
```

```tspp expected
interface Foo extends Bar, Baz {}
```

### interface with property

Interfaces with members expand to multiple lines.
Properties use trailing semicolons.

```tspp
interface Foo { x: number }
```

```tspp expected
interface Foo {
    x: number;
}
```

### interface with multiple properties

Each property goes on its own line with a trailing semicolon.

```tspp
interface Foo { x: number; y: string; z: boolean }
```

```tspp expected
interface Foo {
    x: number;
    y: string;
    z: boolean;
}
```

### interface with method signature

Method signatures use declaration semicolons like other interface members.

```tspp
interface Foo { bar(): void }
```

```tspp expected
interface Foo {
    bar(): void;
}
```

### interface with static method signature

Static method signatures keep their modifier attached to the method.

```tspp
interface Result { static fromError(error: E): this }
```

```tspp expected
interface Result {
    static fromError(error: E): this;
}
```

### interface with method parameters

Method parameters follow standard parameter formatting rules.

```tspp
interface Foo { add(a: number, b: number): number }
```

```tspp expected
interface Foo {
    add(a: number, b: number): number;
}
```

## Export

### exported interface

The `export` keyword precedes the interface declaration.

```tspp
export interface Foo { x: number }
```

```tspp expected
export interface Foo {
    x: number;
}
```

## Line Breaking

### interface with multiline extends

Long extended interface lists break before the keyword and indent each extended type.

```tspp line-width=60
interface Foo extends VeryLongBaseInterfaceNameOne, VeryLongBaseInterfaceNameTwo, VeryLongBaseInterfaceNameThree { value: string }
```

```tspp expected
interface Foo
    extends
        VeryLongBaseInterfaceNameOne,
        VeryLongBaseInterfaceNameTwo,
        VeryLongBaseInterfaceNameThree
{
    value: string;
}
```

### interface with many type params breaks

When type parameters exceed the line width, they break to multiple lines.

```tspp line-width=40
interface Container<VeryLongType, AnotherType, ThirdType> { }
```

Each type parameter goes on its own line with a trailing comma.

```tspp expected
interface Container<
    VeryLongType,
    AnotherType,
    ThirdType,
>
{}
```

## Interface Members

### interface with mixed members

Properties and method signatures both use declaration semicolons.

```tspp
interface User { id: number; name: string; email?: string; getName(): string; setName(name: string): void }
```

```tspp expected
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

```tspp
/// Represents a point in 2D space.
interface Point { x: number; y: number }
```

```tspp expected
/// Represents a point in 2D space.
interface Point {
    x: number;
    y: number;
}
```
