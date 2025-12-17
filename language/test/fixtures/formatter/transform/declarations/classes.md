# Class Declarations

Tests for class declaration formatting.

## Basic Classes

### simple class

Extra whitespace in class declarations should be normalized.

```ds
class   Foo   {   }
```

Empty class bodies stay on one line with internal spacing.

```ds expected
class Foo { }
```

### class with extends

The `extends` clause should have single spaces around it.

```ds
class   Foo   extends   Bar   {   }
```

```ds expected
class Foo extends Bar { }
```

### class with implements

Multiple implemented interfaces are separated by comma and space.

```ds
class   Foo   implements   Bar  ,  Baz   {   }
```

```ds expected
class Foo implements Bar, Baz { }
```

### class with generic

Generic type parameters have no internal spacing.

```ds
class   Foo  <  T  >   {   }
```

```ds expected
class Foo<T> { }
```

### class with field

Classes with members expand to multiple lines with fields getting trailing commas.

```ds
class Foo { x: number }
```

```ds expected
class Foo {
    x: number,
}
```

### class with multiple fields

Each field goes on its own line with a trailing comma.

```ds
class Foo { x: number; y: string; z: boolean }
```

```ds expected
class Foo {
    x: number,
    y: string,
    z: boolean,
}
```

### class with method

Methods expand their bodies to multiple lines when they contain statements.

```ds
class Foo { bar() { return 1 } }
```

```ds expected
class Foo {
    bar() {
        return 1
    }
}
```

### class with constructor

Constructor bodies follow the same rules as method bodies.

```ds
class Foo { constructor(x: number) { this.x = x } }
```

```ds expected
class Foo {
    constructor(x: number) {
        this.x = x
    }
}
```

## Field Modifiers

### public field

Visibility modifiers are preserved before the field name.

```ds
class Foo { public x: number }
```

```ds expected
class Foo {
    public x: number,
}
```

### private field

Private fields use the `private` keyword.

```ds
class Foo { private x: number }
```

```ds expected
class Foo {
    private x: number,
}
```

### protected field

Protected fields are accessible to subclasses.

```ds
class Foo { protected x: number }
```

```ds expected
class Foo {
    protected x: number,
}
```

### private hash field

Destack uses keyword-based visibility rather than JavaScript's `#` syntax.

```ds
class Foo { private x: number }
```

```ds expected
class Foo {
    private x: number,
}
```

### readonly field

The `readonly` modifier prevents field reassignment.

```ds
class Foo { readonly x: number }
```

```ds expected
class Foo {
    readonly x: number,
}
```

### static field

Static fields belong to the class rather than instances.

```ds
class Foo { static count: number = 0 }
```

```ds expected
class Foo {
    static count: number = 0,
}
```

### field with initializer

Field initializers use `=` with spaces around it.

```ds
class Foo { x: number = 42 }
```

```ds expected
class Foo {
    x: number = 42,
}
```

## Methods

### simple method

Simple single-statement method bodies stay on one line.

```ds
class Foo { bar(): void { console.log("hello") } }
```

```ds expected
class Foo {
    bar(): void { console.log("hello") }
}
```

### method with parameters

Method parameters follow function parameter formatting rules.

```ds
class Foo { add(a: number, b: number): number { return a + b } }
```

```ds expected
class Foo {
    add(a: number, b: number): number {
        return a + b
    }
}
```

### async method

The `async` keyword precedes the method name.

```ds
class Foo { async fetch(): Promise<Data> { return await getData() } }
```

```ds expected
class Foo {
    async fetch(): Promise<Data> {
        return await getData()
    }
}
```

### static method

Static methods belong to the class rather than instances.

```ds
class Foo { static create(): Foo { return new Foo() } }
```

```ds expected
class Foo {
    static create(): Foo {
        return new Foo()
    }
}
```

### getter

Getters use the `get` keyword before the property name.

```ds
class Foo { get value(): number { return this._value } }
```

```ds expected
class Foo {
    get value(): number {
        return this._value
    }
}
```

### setter

Setters use the `set` keyword and take exactly one parameter.

```ds
class Foo { set value(v: number) { this._value = v } }
```

```ds expected
class Foo {
    set value(v: number) {
        this._value = v
    }
}
```

### getter and setter pair

Getter and setter pairs are formatted as separate methods.

```ds
class Foo { get x(): number { return this._x } set x(v: number) { this._x = v } }
```

```ds expected
class Foo {
    get x(): number {
        return this._x
    }
    set x(v: number) {
        this._x = v
    }
}
```

## Generics

### generic class

Generic classes have type parameters in angle brackets.

```ds
class Container<T> { value: T }
```

```ds expected
class Container<T> {
    value: T,
}
```

### generic class with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
class Container<T: Comparable> { value: T }
```

```ds expected
class Container<T: Comparable> {
    value: T,
}
```

### generic class with multiple type params

Multiple type parameters are separated by commas.

```ds
class Pair<K, V> { key: K; value: V }
```

```ds expected
class Pair<K, V> {
    key: K,
    value: V,
}
```

## Inheritance

### class extends

Subclasses use `extends` to inherit from a base class.

```ds
class Dog extends Animal { bark() { } }
```

```ds expected
class Dog extends Animal {
    bark() { }
}
```

### class implements

Classes use `implements` to satisfy interface contracts.

```ds
class Dog implements Animal { makeSound() { } }
```

```ds expected
class Dog implements Animal {
    makeSound() { }
}
```

### class extends and implements

A class can both extend a base class and implement interfaces.

```ds
class Dog extends Pet implements Animal, Named { name: string }
```

```ds expected
class Dog extends Pet implements Animal, Named {
    name: string,
}
```

### class with super call

Simple single-statement constructor bodies stay on one line.

```ds
class Dog extends Animal { constructor() { super() } }
```

```ds expected
class Dog extends Animal {
    constructor() { super() }
}
```

## Decorators

### decorated class

Decorators appear on their own line before the class.

```ds
@Component
class MyComponent { }
```

```ds expected
@Component
class MyComponent { }
```

### decorator with arguments

Decorator arguments follow function call formatting.

```ds
@Component({ selector: "my-component" })
class MyComponent { }
```

```ds expected
@Component({ selector: "my-component" })
class MyComponent { }
```

### multiple decorators

Decorator calls without arguments lose their empty parentheses.

```ds
@Injectable()
@Singleton
class Service { }
```

```ds expected
@Injectable
@Singleton
class Service { }
```

### decorated field

Field decorators appear on their own line above the field.

```ds
class Foo { @observable x: number }
```

```ds expected
class Foo {
    @observable
    x: number,
}
```

### decorated method

Method decorators appear on their own line above the method.

```ds
class Foo { @memoize compute(): number { return 42 } }
```

```ds expected
class Foo {
    @memoize
    compute(): number {
        return 42
    }
}
```

## Export

### exported class

The `export` keyword precedes the class declaration.

```ds
export class Foo { }
```

```ds expected
export class Foo { }
```

### export default class

Default exports use `export default` before the class.

```ds
export default class Handler { }
```

```ds expected
export default class Handler { }
```

## Line Breaking

### class with many type params breaks

When type parameters exceed the line width, they break to multiple lines.

```ds line-width=40
class Container<VeryLongType, AnotherType, ThirdType> { }
```

Each type parameter goes on its own line with a trailing comma.

```ds expected
class Container<
    VeryLongType,
    AnotherType,
    ThirdType,
> { }
```

### class with long implements breaks

Destack uses parentheses for multi-line implements clauses.

```ds line-width=50
class MyClass implements FirstInterface, SecondInterface, ThirdInterface { }
```

```ds expected
class MyClass implements (
    FirstInterface,
    SecondInterface,
    ThirdInterface,
) { }
```

## Complex Classes

### class with mixed members

Fields have trailing commas, methods do not.

```ds
class Person { name: string; constructor(name: string) { this.name = name } greet(): string { return `Hello, ${this.name}` } }
```

```ds expected
class Person {
    name: string,
    constructor(name: string) {
        this.name = name
    }
    greet(): string {
        return `Hello, ${this.name}`
    }
}
```

### class with static block

Single-statement static blocks stay on one line.

```ds
class Config { static { Config.init() } static init() { } }
```

```ds expected
class Config {
    static { Config.init() }
    static init() { }
}
```

## Documentation

### class with doc comment

Doc comments are preserved above the class declaration.

```ds
/// A point in 2D space.
class Point { x: number; y: number }
```

```ds expected
/// A point in 2D space.
class Point {
    x: number,
    y: number,
}
```

### _field with doc comment

Inline doc comments before fields are moved to their own line (parser issue - doc comment eats line).

```ds
class Point { /// The x coordinate. x: number }
```

```ds expected
class Point {
    /// The x coordinate.
    x: number,
}
```
