# Class Declarations

Class fixtures cover class heads, members, modifiers, decorators, and heritage clauses.

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

```ts:main.ts
export class ClassTest extends Modal<
  // comment
  string | number | undefined
> {
}
```

```ts expected
export class ClassTest extends Modal<
    // comment
    string | number | undefined
> {}
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

### constructor with parameter properties

Parameter properties keep visibility and readonly modifiers.

```ts:main.ts
class Foo { constructor(public x: number, private readonly y: string) { } }
```

```ts expected
class Foo {
    constructor(
        public x: number,
        private readonly y: string,
    ) {}
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
    public x: number;
}
```

### private field

Private fields use the `private` keyword.

```ds
class Foo { private x: number }
```

```ds expected
class Foo {
    private x: number;
}
```

### protected field

Protected fields are accessible to subclasses.

```ds
class Foo { protected x: number }
```

```ds expected
class Foo {
    protected x: number;
}
```

### private hash field

Private shorthand fields keep the `#` prefix.

```ds
class Counter { #value: int32 = 0 }
```

```ds expected
class Counter {
    #value: int32 = 0;
}
```

### private member access

Private member access keeps the `.#` syntax and formats like other member chains.

```ds
class Counter { #value: int32 = 0; get(): int32 { return this.#value } }
```

```ds expected
class Counter {
    #value: int32 = 0;
    get(): int32 {
        return this.#value;
    }
}
```

### private member call

Private method calls keep the `.#` token and follow standard call formatting.

```ds
class Counter { #next(): int32 { return 1 } get(): int32 { return this.#next() } }
```

```ds expected
class Counter {
    #next(): int32 {
        return 1;
    }
    get(): int32 {
        return this.#next();
    }
}
```

### readonly field

The `readonly` modifier prevents field reassignment.

```ds
class Foo { readonly x: number }
```

```ds expected
class Foo {
    readonly x: number;
}
```

### static field

Static fields belong to the class rather than instances.

```ds
class Foo { static count: number = 0 }
```

```ds expected
class Foo {
    static count: number = 0;
}
```

### field with initializer

Field initializers use `=` with spaces around it.

```ds
class Foo { x: number = 42 }
```

```ds expected
class Foo {
    x: number = 42;
}
```

## TypeScript Class Forms

### TypeScript class fields use semicolons

TypeScript class fields use semicolons instead of commas.

```ts:main.ts
class Foo { x: number; y: string }
```

```ts expected
class Foo {
    x: number;
    y: string;
}
```

### TypeScript accessor field

Accessor fields keep the `accessor` keyword and use semicolons.

```ts:main.ts
class Box { accessor value = 1 }
```

```ts expected
class Box {
    accessor value = 1;
}
```

### TypeScript private hash members

Private hash members keep the `#` prefix and use semicolons.

```ts:main.ts
class Foo { #count: number; #reset() { } static #value = 1 }
```

```ts expected
class Foo {
    #count: number;
    #reset() {}
    static #value = 1;
}
```

### TypeScript class quoted keys

Quoted class members preserve their original quoting unless quote-props mode says otherwise.

```ts:main.ts
class Config { "normal" = 1; "data-id" = 2; "default"() { } }
```

```ts expected
class Config {
    "normal" = 1;
    "data-id" = 2;
    default() {}
}
```

### Destack class quoted members

Quoted class members preserve their original quoting unless quote-props mode says otherwise.

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

### TypeScript class quote props consistent

Consistent quote props quotes all keys when any require quotes.

```ts:main.ts quote-props=consistent
class Options { normal = 1; "data-id" = 2; "default"() { } }
```

```ts expected
class Options {
    "normal" = 1;
    "data-id" = 2;
    "default"() {}
}
```

### Destack class quote props consistent

Consistent quote props applies to shared class member syntax.

```ds quote-props=consistent
class Options { normal = 1; "data-id" = 2; "default"() { } }
```

```ds expected
class Options {
    "normal" = 1;
    "data-id" = 2;
    "default"() {}
}
```

### TypeScript class unicode methods

Unicode method names stay quoted and normalize quote style.

```ts:main.ts
class A { 'x・'() {} 'x･'() {} }
```

```ts expected
class A {
    "x・"() {}
    "x･"() {}
}
```

### TypeScript abstract class preserves keyword

Abstract classes keep the `abstract` modifier.

```ts:main.ts
abstract class Foo { abstract bar(): void }
```

```ts expected
abstract class Foo {
    abstract bar(): void;
}
```

### TypeScript override method preserves keyword

Override methods keep the `override` modifier.

```ts:main.ts
class Base { greet(): void { } }
class Child extends Base { override greet(): void { } }
```

```ts expected
class Base {
    greet(): void {}
}
class Child extends Base {
    override greet(): void {}
}
```

### TypeScript abstract override method preserves keywords

Abstract override methods keep both modifiers.

```ts:main.ts
abstract class Base { abstract greet(): void }
abstract class Child extends Base { abstract override greet(): void }
```

```ts expected
abstract class Base {
    abstract greet(): void;
}
abstract class Child extends Base {
    abstract override greet(): void;
}
```

### TypeScript declaration method signatures use semicolons

TypeScript declaration class signatures end with semicolons.

```ts:main.d.ts
declare class Foo { bar(): void }
```

```ts expected
declare class Foo {
    bar(): void;
}
```

## Methods

### method declaration

Simple single-statement method bodies stay on one line.

```ds
class Foo { bar(): void { console.log("hello") } }
```

```ds expected
class Foo {
    bar(): void {
        console.log("hello");
    }
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
        return a + b;
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
        return await getData();
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
        return new Foo();
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
        return this._value;
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
        this._value = v;
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
        return this._x;
    }
    set x(v: number) {
        this._x = v;
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
    value: T;
}
```

### generic class with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
class Container<T: Comparable> { value: T }
```

```ds expected
class Container<T: Comparable> {
    value: T;
}
```

### generic class with multiple type params

Multiple type parameters are separated by commas.

```ds
class Pair<K, V> { key: K; value: V }
```

```ds expected
class Pair<K, V> {
    key: K;
    value: V;
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
    bark() {}
}
```

### class implements

Classes use `implements` to satisfy interface contracts.

```ds
class Dog implements Animal { makeSound() { } }
```

```ds expected
class Dog implements Animal {
    makeSound() {}
}
```

### class extends and implements

A class can both extend a base class and implement interfaces.

```ds
class Dog extends Pet implements Animal, Named { name: string }
```

```ds expected
class Dog extends Pet implements Animal, Named {
    name: string;
}
```

### class with super call

Simple single-statement constructor bodies stay on one line.

```ds
class Dog extends Animal { constructor() { super() } }
```

```ds expected
class Dog extends Animal {
    constructor() {
        super();
    }
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
class MyComponent {}
```

### decorator with arguments

Decorator arguments follow function call formatting.

```ds
@Component({ selector: "my-component" })
class MyComponent { }
```

```ds expected
@Component({ selector: "my-component" })
class MyComponent {}
```

### multiple decorators

Decorator calls keep their empty parentheses.

```ds
@Injectable()
@Singleton
class Service { }
```

```ds expected
@Injectable()
@Singleton
class Service {}
```

### decorator expressions with calls

Complex decorator expressions use parentheses for clarity.

```ds
@factory().decorator
@factory().decorator()
@decorator().member
@decorator().member()
class Service { }
```

```ds expected
@(factory().decorator)
@(factory().decorator())
@(decorator().member)
@(decorator().member())
class Service {}
```

### decorator instantiation expressions

Decorator instantiation expressions use parentheses.

```ds
@decorator<T>
class Service { }
```

```ds expected
@(decorator<T>)
class Service {}
```

### decorated field

Field decorators appear on their own line above the field.

```ds
class Foo { @observable x: number }
```

```ds expected
class Foo {
    @observable
    x: number;
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
        return 42;
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

```ts:main.ts
export @logged class Handler {}
```

```ts expected
export
@logged
class Handler {}
```

### decorator before export clause

Leading decorators stay before `export` when they start before the export keyword.

```ts:main.ts
@logged export class Handler {}
```

```ts expected
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
> {}
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
        ThirdInterface {}
```

## Complex Classes

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
