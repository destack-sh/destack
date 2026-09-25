# Class Declarations

## Class Forms

### class declaration

Extra whitespace in class declarations should be normalized.

```tspp
class   Foo   {   }
```

Empty class bodies stay on one line with internal spacing.

```tspp expected
class Foo {}
```

### class with extends

The `extends` clause should have single spaces around it.

```tspp
class   Foo   extends   Bar   {   }
```

```tspp expected
class Foo extends Bar {}
```

### class extends with commented type arguments

Comments inside class heritage type arguments keep the type arguments multiline.

```tspp:main.tspp
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {
}
```

```tspp expected
export class ClassTest extends Modal<
    // comment
    string | number | undefined
>
{}
```

### class with implements

Multiple implemented interfaces are separated by comma and space.

```tspp
class   Foo   implements   Bar  ,  Baz   {   }
```

```tspp expected
class Foo implements Bar, Baz {}
```

### class with generic

Generic type parameters have no internal spacing.

```tspp
class   Foo  <  T  >   {   }
```

```tspp expected
class Foo<T> {}
```

### class with field

Classes with members expand to multiple lines with fields ending in semicolons.

```tspp
class Foo { x: number }
```

```tspp expected
class Foo {
    x: number;
}
```

### class with multiple fields

Each field goes on its own line with a trailing semicolon.

```tspp
class Foo { x: number; y: string; z: boolean }
```

```tspp expected
class Foo {
    x: number;
    y: string;
    z: boolean;
}
```

### class with method

Methods expand their bodies to multiple lines when they contain statements.

```tspp
class Foo { bar() { return 1 } }
```

```tspp expected
class Foo {
    bar() {
        return 1;
    }
}
```

### class method tail expression

Value-returning methods keep terminal expressions semicolonless.

```tspp
class Foo { value(): number { this.current } }
```

```tspp expected
class Foo {
    value(): number {
        this.current
    }
}
```

### class constructor and setter bodies

Constructors and setters keep terminal expressions as statements.

```tspp
class Foo { constructor() { initialize() } set value(next: number) { this.current = next } }
```

```tspp expected
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

```tspp
class Foo { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { this.limit } else { doubled } } }
```

```tspp expected
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

```tspp
class Foo { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { const capped = this.limit - 1; capped } else { const returned = doubled + 1; returned } } }
```

```tspp expected
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

```tspp
class Foo { constructor(x: number) { this.x = x } }
```

```tspp expected
class Foo {
    constructor(x: number) {
        this.x = x;
    }
}
```

## Class Forms

### class fields use semicolons

Class fields use semicolons instead of commas.

```tspp:main.tspp
class Foo { x: number; y: string }
```

```tspp expected
class Foo {
    x: number;
    y: string;
}
```

### optional field suffix

Optional field suffixes stay attached to the field name.

```tspp:main.tspp
class Foo { maybe?: string }
```

```tspp expected
class Foo {
    maybe?: string;
}
```

### accessor field

Accessor fields keep the `accessor` keyword and use semicolons.

```tspp:main.tspp
class Box { accessor value = 1 }
```

```tspp expected
class Box {
    accessor value = 1;
}
```

### class quoted keys

Quoted class members preserve required quotes and remove unnecessary keyword quotes.

```tspp:main.tspp
class Config { "normal" = 1; "data-id" = 2; "default"() { } }
```

```tspp expected
class Config {
    "normal" = 1;
    "data-id" = 2;
    default() {}
}
```

### TS++ class quoted members

Quoted class members preserve required quotes and remove unnecessary keyword quotes.

```tspp
class Config { "normal" = 1; "data-id" = 2; "default"() { } }
```

```tspp expected
class Config {
    "normal" = 1;
    "data-id" = 2;
    default() {}
}
```

### class unicode methods

Unicode method names stay quoted and normalize quote style.

```tspp:main.tspp
class A { 'x・'() {} 'x･'() {} }
```

```tspp expected
class A {
    "x・"() {}
    "x･"() {}
}
```

### abstract class preserves keyword

Abstract classes keep the `abstract` modifier.

```tspp:main.tspp
abstract class Foo { abstract bar(): void }
```

```tspp expected
abstract class Foo {
    abstract bar(): void;
}
```

### override method preserves keyword

Override methods keep the `override` modifier.

```tspp:main.tspp
class Base { greet(): void { } }
class Child extends Base { override greet(): void { } }
```

```tspp expected
class Base {
    greet(): void {}
}
class Child extends Base {
    override greet(): void {}
}
```

### abstract override method preserves keywords

Abstract override methods keep both modifiers.

```tspp:main.tspp
abstract class Base { abstract greet(): void }
abstract class Child extends Base { abstract override greet(): void }
```

```tspp expected
abstract class Base {
    abstract greet(): void;
}
abstract class Child extends Base {
    abstract override greet(): void;
}
```

### final class

`final` prints before the class keyword.

```tspp
final class Service { start(): void {} }
```

```tspp expected
final class Service {
    start(): void {}
}
```

### virtual method

`virtual` prints before the method key.

```tspp
class Widget { virtual render(): void {} }
```

```tspp expected
class Widget {
    virtual render(): void {}
}
```

### declaration method signatures use semicolons

Declaration class signatures end with semicolons.

```tspp:main.d.tspp
declare class Foo { bar(): void }
```

```tspp expected
declare class Foo {
    bar(): void;
}
```

## Export

### exported class

The `export` keyword precedes the class declaration.

```tspp
export class Foo { }
```

```tspp expected
export class Foo {}
```

### export default class

Default exports use `export default` before the class.

```tspp
export default class Handler { }
```

```tspp expected
export default class Handler {}
```

### export before decorator clause

Exported class decorators stay after `export` when they start after the export keyword.

```tspp:main.tspp
export @logged class Handler {}
```

```tspp expected
export
@logged
class Handler {}
```

### decorator before export clause

Leading decorators stay before `export` when they start before the export keyword.

```tspp:main.tspp
@logged export class Handler {}
```

```tspp expected
@logged
export class Handler {}
```

## Line Breaking

### class with many type params breaks

When type parameters exceed the line width, they break to multiple lines.

```tspp line-width=40
class Container<VeryLongType, AnotherType, ThirdType> { }
```

Each type parameter goes on its own line with a trailing comma.

```tspp expected
class Container<
    VeryLongType,
    AnotherType,
    ThirdType,
>
{}
```

### class with long implements breaks

Long `implements` clauses break across lines without wrapper parentheses.

```tspp line-width=50
class MyClass implements FirstInterface, SecondInterface, ThirdInterface { }
```

```tspp expected
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

```tspp
class Person { name: string; constructor(name: string) { this.name = name } greet(): string { return `Hello, ${this.name}` } }
```

```tspp expected
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

```tspp
class Config { static { Config.init() } static init() { } }
```

```tspp expected
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

```tspp
/// A point in 2D space.
class Point { x: number; y: number }
```

```tspp expected
/// A point in 2D space.
class Point {
    x: number;
    y: number;
}
```

### field with doc comment

Doc comments before fields remain attached.

```tspp
class Point {
    /// The x coordinate.
    x: number
}
```

```tspp expected
class Point {
    /// The x coordinate.
    x: number;
}
```

Combined declaration fixtures cover documentation, decorators, heritage clauses, and member bodies together.

## Classes

### documented decorated exported class with heritage

Documentation, decorators, exports, heritage clauses, and decorated members keep their relative order.

```tspp:main.tspp line-width=80
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

```tspp expected
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
