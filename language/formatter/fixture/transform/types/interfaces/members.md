# Interface Members

## Optional Members

### optional property

Optional properties use `?` after the property name.

```tspp
interface Foo { x?: number }
```

```tspp expected
interface Foo {
    x?: number;
}
```

### optional method

TS++ uses `method?()` syntax for optional methods.

```tspp
interface Foo { bar?(): void }
```

```tspp expected
interface Foo {
    bar?(): void;
}
```

## Readonly Members

### readonly property

The `readonly` modifier prevents property reassignment.

```tspp
interface Foo { readonly x: number }
```

```tspp expected
interface Foo {
    readonly x: number;
}
```

## Associated Members

### associated type modifiers

Associated type modifiers are preserved.

```tspp
interface Foo { abstract type Item override type Output = string }
```

```tspp expected
interface Foo {
    abstract type Item;
    override type Output = string;
}
```

### associated constant modifiers

Associated constant modifiers are preserved.

```tspp
interface Foo { abstract const Size: uint override const Count: uint = 2 }
```

```tspp expected
interface Foo {
    abstract const Size: uint;
    override const Count: uint = 2;
}
```
