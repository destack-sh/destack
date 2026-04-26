# Interface Declarations

Tests for interface declaration formatting.

## Basic Interfaces

### simple interface

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

Interfaces with members expand to multiple lines. Properties use trailing semicolons.

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

## Optional Members

### optional property

Optional properties use `?` after the property name.

```ds
interface Foo { x?: number }
```

```ds expected
interface Foo {
    x?: number;
}
```

### optional method

Destack uses `method?()` syntax for optional methods.

```ds
interface Foo { bar?(): void }
```

```ds expected
interface Foo {
    bar?(): void;
}
```

## Readonly Members

### readonly property

The `readonly` modifier prevents property reassignment.

```ds
interface Foo { readonly x: number }
```

```ds expected
interface Foo {
    readonly x: number;
}
```

## Generics

### generic interface

Generic interfaces have type parameters in angle brackets.

```ds
interface Container<T> { value: T }
```

```ds expected
interface Container<T> {
    value: T;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
interface Container<T: Comparable> { value: T }
```

```ds expected
interface Container<T: Comparable> {
    value: T;
}
```

### variance parameters

Variance modifiers precede type parameter names.

```ds
interface   Box< in  T , out U > { get(): U; set(value: T): void }
```

```ds expected
interface Box<in T, out U> {
    get(): U;
    set(value: T): void;
}
```

### multiple type parameters

Multiple type parameters are separated by commas.

```ds
interface Map<K, V> { get(key: K): V; set(key: K, value: V): void }
```

```ds expected
interface Map<K, V> {
    get(key: K): V;
    set(key: K, value: V): void;
}
```

## Index Signatures

### string index signature

String index signatures allow dictionary-like access.

```ds
interface Dict { [key: string]: number }
```

```ds expected
interface Dict {
    [key: string]: number;
}
```

### number index signature

Number index signatures allow array-like access.

```ds
interface ArrayLike { [index: number]: string }
```

```ds expected
interface ArrayLike {
    [index: number]: string;
}
```

### mixed index and properties

Index signatures can coexist with regular properties.

```ds
interface Dict { [key: string]: number; length: number }
```

```ds expected
interface Dict {
    [key: string]: number;
    length: number;
}
```

## Call and Construct Signatures

### call signature

Call signatures make an interface callable like a function.

```ds
interface Callable { (x: number): number }
```

```ds expected
interface Callable {
    (x: number): number;
}
```

### construct signature

Construct signatures allow using `new` with the interface.

```ds
interface Constructor { new(x: number): Foo }
```

```ds expected
interface Constructor {
    new (x: number): Foo;
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

## Complex Interfaces

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

## TypeScript Signatures

### new signature in interface

TypeScript `new` signatures keep a space before parameter lists.

```ts:main.ts
interface Creator { new(...args): Foo }
```

```ts expected
interface Creator {
    new (...args): Foo;
}
```

### call signature in interface

Call signatures format without a name and include semicolons.

```ts:main.ts
interface Callable { (...args): Foo }
```

```ts expected
interface Callable {
    (...args): Foo;
}
```
