# Class Declarations

## Class Forms

### class declaration

Extra whitespace in class declarations should be normalized.

```ds
class   Foo   {   }
```

Empty class bodies stay on one line with internal spacing.

```ds expected
class Foo {}
```

### class with extends

The `extends` clause should have single spaces around it.

```ds
class   Foo   extends   Bar   {   }
```

```ds expected
class Foo extends Bar {}
```

### class extends with commented type arguments

Comments inside class heritage type arguments keep the type arguments multiline.

```ds:main.ds
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {
}
```

```ds expected
export class ClassTest extends Modal<
    // comment
    string | number | undefined
>
{}
```

### class with implements

Multiple implemented interfaces are separated by comma and space.

```ds
class   Foo   implements   Bar  ,  Baz   {   }
```

```ds expected
class Foo implements Bar, Baz {}
```

### class with generic

Generic type parameters have no internal spacing.

```ds
class   Foo  <  T  >   {   }
```

```ds expected
class Foo<T> {}
```

### class with field

Classes with members expand to multiple lines with fields ending in semicolons.

```ds
class Foo { x: number }
```

```ds expected
class Foo {
    x: number;
}
```

### class with multiple fields

Each field goes on its own line with a trailing semicolon.

```ds
class Foo { x: number; y: string; z: boolean }
```

```ds expected
class Foo {
    x: number;
    y: string;
    z: boolean;
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
        return 1;
    }
}
```

### class method tail expression

Value-returning methods keep terminal expressions semicolonless.

```ds
class Foo { value(): number { this.current } }
```

```ds expected
class Foo {
    value(): number {
        this.current
    }
}
```

### class constructor and setter bodies

Constructors and setters keep terminal expressions as statements.

```ds
class Foo { constructor() { initialize() } set value(next: number) { this.current = next } }
```

```ds expected
class Foo {
    constructor() {
        initialize();
    }
    set value(next: number) {
        this.current = next;
    }
}
```

### class method short nested value tail

Short value-returning methods expand nested control-flow tails.

```ds
class Foo { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { this.limit } else { doubled } } }
```

```ds expected
class Foo {
    value(next: number): number {
        const doubled = next * 2;
        if (doubled > this.limit) {
            this.limit
        } else {
            doubled
        }
    }
}
```

### class method expanded nested value tail

Value-returning methods preserve expression tails through nested control flow.

```ds
class Foo { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { const capped = this.limit - 1; capped } else { const returned = doubled + 1; returned } } }
```

```ds expected
class Foo {
    value(next: number): number {
        const doubled = next * 2;
        if (doubled > this.limit) {
            const capped = this.limit - 1;
            capped
        } else {
            const returned = doubled + 1;
            returned
        }
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
        this.x = x;
    }
}
```

## Class Forms

### class fields use semicolons

Class fields use semicolons instead of commas.

```ds:main.ds
class Foo { x: number; y: string }
```

```ds expected
class Foo {
    x: number;
    y: string;
}
```

### field suffixes

Optional and definite field suffixes stay attached to the field name.

```ds:main.ds
class Foo { ready!: boolean; maybe?: string; accessor value!: number }
```

```ds expected
class Foo {
    ready!: boolean;
    maybe?: string;
    accessor value!: number;
}
```

### accessor field

Accessor fields keep the `accessor` keyword and use semicolons.

```ds:main.ds
class Box { accessor value = 1 }
```

```ds expected
class Box {
    accessor value = 1;
}
```

### class quoted keys

Quoted class members preserve required quotes and remove unnecessary keyword quotes.

```ds:main.ds
class Config { "normal" = 1; "data-id" = 2; "default"() { } }
```

```ds expected
class Config {
    "normal" = 1;
    "data-id" = 2;
    default() {}
}
```

### Destack class quoted members

Quoted class members preserve required quotes and remove unnecessary keyword quotes.

```ds
class Config { "normal" = 1; "data-id" = 2; "default"() { } }
```

```ds expected
class Config {
    "normal" = 1;
    "data-id" = 2;
    default() {}
}
```

### class unicode methods

Unicode method names stay quoted and normalize quote style.

```ds:main.ds
class A { 'x・'() {} 'x･'() {} }
```

```ds expected
class A {
    "x・"() {}
    "x･"() {}
}
```

### abstract class preserves keyword

Abstract classes keep the `abstract` modifier.

```ds:main.ds
abstract class Foo { abstract bar(): void }
```

```ds expected
abstract class Foo {
    abstract bar(): void;
}
```

### override method preserves keyword

Override methods keep the `override` modifier.

```ds:main.ds
class Base { greet(): void { } }
class Child extends Base { override greet(): void { } }
```

```ds expected
class Base {
    greet(): void {}
}
class Child extends Base {
    override greet(): void {}
}
```

### abstract override method preserves keywords

Abstract override methods keep both modifiers.

```ds:main.ds
abstract class Base { abstract greet(): void }
abstract class Child extends Base { abstract override greet(): void }
```

```ds expected
abstract class Base {
    abstract greet(): void;
}
abstract class Child extends Base {
    abstract override greet(): void;
}
```

### final class

`final` prints before the class keyword.

```ds
final class Service { start(): void {} }
```

```ds expected
final class Service {
    start(): void {}
}
```

### virtual method

`virtual` prints before the method key.

```ds
class Widget { virtual render(): void {} }
```

```ds expected
class Widget {
    virtual render(): void {}
}
```

### declaration method signatures use semicolons

Declaration class signatures end with semicolons.

```ds:main.d.ds
declare class Foo { bar(): void }
```

```ds expected
declare class Foo {
    bar(): void;
}
```

## Export

### exported class

The `export` keyword precedes the class declaration.

```ds
export class Foo { }
```

```ds expected
export class Foo {}
```

### export default class

Default exports use `export default` before the class.

```ds
export default class Handler { }
```

```ds expected
export default class Handler {}
```

### export before decorator clause

Exported class decorators stay after `export` when they start after the export keyword.

```ds:main.ds
export @logged class Handler {}
```

```ds expected
export
@logged
class Handler {}
```

### decorator before export clause

Leading decorators stay before `export` when they start before the export keyword.

```ds:main.ds
@logged export class Handler {}
```

```ds expected
@logged
export class Handler {}
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
>
{}
```

### class with long implements breaks

Long `implements` clauses break across lines without wrapper parentheses.

```ds line-width=50
class MyClass implements FirstInterface, SecondInterface, ThirdInterface { }
```

```ds expected
class MyClass
    implements
        FirstInterface,
        SecondInterface,
        ThirdInterface
{}
```

## Class Members

### class with mixed members

Fields use trailing semicolons, and methods expand as full declarations.

```ds
class Person { name: string; constructor(name: string) { this.name = name } greet(): string { return `Hello, ${this.name}` } }
```

```ds expected
class Person {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
    greet(): string {
        return `Hello, ${this.name}`;
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
    static {
        Config.init();
    }
    static init() {}
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
    x: number;
    y: number;
}
```

### field with doc comment

Doc comments before fields remain attached.

```ds
class Point {
    /// The x coordinate.
    x: number
}
```

```ds expected
class Point {
    /// The x coordinate.
    x: number;
}
```

Combined declaration fixtures cover documentation, decorators, heritage clauses, and member bodies together.

## Classes

### documented decorated exported class with heritage

Documentation, decorators, exports, heritage clauses, and decorated members keep their relative order.

```ds:main.ds line-width=80
/**
 * Stores values.
 * @typeParam T value type
 */
@entity
export class Store<T> extends Base<T> implements Reader<T> {
  /**
   * Current value.
   * @returns stored value
   */
  @tracked
  get value(): T { return this.current }
}
```

```ds expected
/// Stores values.
///
/// @typeParam T - value type
@entity
export class Store<T> extends Base<T> implements Reader<T> {
    /// Current value.
    ///
    /// @returns stored value
    @tracked
    get value(): T {
        return this.current;
    }
}
```
