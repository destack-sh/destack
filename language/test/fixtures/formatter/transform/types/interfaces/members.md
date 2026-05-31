# Interface Members

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

## Associated Members

### associated type modifiers

Associated type modifiers are preserved.

```ds
interface Foo { abstract type Item override type Output = string }
```

```ds expected
interface Foo {
    abstract type Item;
    override type Output = string;
}
```

### associated constant modifiers

Associated constant modifiers are preserved.

```ds
interface Foo { abstract comptime const Size: uint override comptime const Count: uint = 2 }
```

```ds expected
interface Foo {
    abstract comptime const Size: uint;
    override comptime const Count: uint = 2;
}
```
