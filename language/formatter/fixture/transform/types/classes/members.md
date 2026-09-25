# Class Members

## Field Modifiers

### public field

Visibility modifiers are preserved before the field name.

```tspp
class Foo { public x: number }
```

```tspp expected
class Foo {
    public x: number;
}
```

### private field

Private fields use the `private` keyword.

```tspp
class Foo { private x: number }
```

```tspp expected
class Foo {
    private x: number;
}
```

### protected field

Protected fields are accessible to subclasses.

```tspp
class Foo { protected x: number }
```

```tspp expected
class Foo {
    protected x: number;
}
```

### readonly field

The `readonly` modifier prevents field reassignment.

```tspp
class Foo { readonly x: number }
```

```tspp expected
class Foo {
    readonly x: number;
}
```

### static field

Static fields belong to the class rather than instances.

```tspp
class Foo { static count: number = 0 }
```

```tspp expected
class Foo {
    static count: number = 0;
}
```

### field with initializer

Field initializers use `=` with spaces around it.

```tspp
class Foo { x: number = 42 }
```

```tspp expected
class Foo {
    x: number = 42;
}
```

## Associated Members

### associated type modifiers

Associated type modifiers are preserved.

```tspp
abstract class Foo { abstract type Item override type Output = string }
```

```tspp expected
abstract class Foo {
    abstract type Item;
    override type Output = string;
}
```

### associated constant modifiers

Associated constant modifiers are preserved.

```tspp
abstract class Foo { abstract const Size: uint; override const Count: uint = 2 }
```

```tspp expected
abstract class Foo {
    abstract const Size: uint;
    override const Count: uint = 2;
}
```

## Methods

### method declaration

Simple single-statement method bodies stay on one line.

```tspp
class Foo { bar(): void { console.log("hello") } }
```

```tspp expected
class Foo {
    bar(): void {
        console.log("hello");
    }
}
```

### method with parameters

Method parameters follow function parameter formatting rules.

```tspp
class Foo { add(a: number, b: number): number { return a + b } }
```

```tspp expected
class Foo {
    add(a: number, b: number): number {
        return a + b;
    }
}
```

### method with long where constraints

Long method where constraints break under the `where` keyword.

```tspp line-width=72
class Store<K: Hash, V> { get<Q: Hash>(key: &readonly Q): V | undefined where K: VeryLongBorrow<Q>, Q: VeryLongEqual<Q>, V: VeryLongClone { return undefined } }
```

```tspp expected
class Store<K: Hash, V> {
    get<Q: Hash>(key: &readonly Q): V | undefined
        where
            K: VeryLongBorrow<Q>,
            Q: VeryLongEqual<Q>,
            V: VeryLongClone {
        return undefined;
    }
}
```

### async method

The `async` keyword precedes the method name.

```tspp
class Foo { async fetch(): Promise<Data> { return await getData() } }
```

```tspp expected
class Foo {
    async fetch(): Promise<Data> {
        return await getData();
    }
}
```

### static method

Static methods belong to the class rather than instances.

```tspp
class Foo { static create(): Foo { return new Foo() } }
```

```tspp expected
class Foo {
    static create(): Foo {
        return new Foo();
    }
}
```

### getter

Getters use the `get` keyword before the property name.

```tspp
class Foo { get value(): number { return this._value } }
```

```tspp expected
class Foo {
    get value(): number {
        return this._value;
    }
}
```

### setter

Setters use the `set` keyword and take exactly one parameter.

```tspp
class Foo { set value(v: number) { this._value = v } }
```

```tspp expected
class Foo {
    set value(v: number) {
        this._value = v;
    }
}
```

### getter and setter pair

Getter and setter pairs are formatted as separate methods.

```tspp
class Foo { get x(): number { return this._x } set x(v: number) { this._x = v } }
```

```tspp expected
class Foo {
    get x(): number {
        return this._x;
    }
    set x(v: number) {
        this._x = v;
    }
}
```
