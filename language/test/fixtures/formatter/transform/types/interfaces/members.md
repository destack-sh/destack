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
