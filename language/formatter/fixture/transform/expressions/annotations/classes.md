# Class Annotations

## Classes

### class field trailing block comment

Trailing block comments on class fields stay with the same field.

```ds:main.ds
class Box {
  first = 1 /* first-tail */
  second = 2
}
```

```ds expected
class Box {
    first = 1; /* first-tail */
    second = 2;
}
```

### decorated field stays on its own line

Body level field decorators stay on their own line above the field.

```ds:main.ds
class Box {
  @observable
  value: number
}
```

```ds expected
class Box {
    @observable
    value: number;
}
```

### decorated method stays on its own line

Body level method decorators stay on their own line above the method.

```ds:main.ds
class Box {
  @memoize
  compute(): number { return 1 }
}
```

```ds expected
class Box {
    @memoize
    compute(): number {
        return 1;
    }
}
```

### decorated accessor stays on its own line

Accessor decorators stay on their own line above the accessor.

```ds:main.ds
class Box {
  @observable accessor value: number
}
```

```ds expected
class Box {
    @observable
    accessor value: number;
}
```

### interleaved method decorator comments stay in order

Comments between stacked method decorators stay interleaved with the same decorator group.

```ds:main.ds
class Box {
  // comment before entity
  @entity
  // comment after entity
  // comment before foo
  @foo(1, 2, 3)
  // comment after foo
  method() {}
}
```

```ds expected
class Box {
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    method() {}
}
```

### abstract accessor type annotation

Abstract accessor members keep type annotations.

```ds:main.ds
abstract class Foo {
  abstract accessor prop7: number;
}
```

```ds expected
abstract class Foo {
    abstract accessor prop7: number;
}
```
