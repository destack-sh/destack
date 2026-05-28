# Class Members

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
