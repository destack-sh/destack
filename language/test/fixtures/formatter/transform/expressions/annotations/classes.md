# Class Annotations

## Classes

### class field trailing block comment

Trailing block comments on class fields stay with the same field.

```ts:main.ts
class Box {
  first = 1 /* first-tail */
  second = 2
}
```

```ts expected
class Box {
    first = 1; /* first-tail */
    second = 2;
}
```

### decorated field stays on its own line

Body level field decorators stay on their own line above the field.

```ts:main.ts
class Box {
  @observable
  value: number
}
```

```ts expected
class Box {
    @observable
    value: number;
}
```

### decorated method stays on its own line

Body level method decorators stay on their own line above the method.

```ts:main.ts
class Box {
  @memoize
  compute(): number { return 1 }
}
```

```ts expected
class Box {
    @memoize
    compute(): number {
        return 1;
    }
}
```

### decorated accessor stays on its own line

Accessor decorators stay on their own line above the accessor.

```ts:main.ts
class Box {
  @observable accessor value: number
}
```

```ts expected
class Box {
    @observable
    accessor value: number;
}
```

### interleaved method decorator comments stay in order

Comments between stacked method decorators stay interleaved with the same decorator group.

```ts:main.ts
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

```ts expected
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

### private accessor hash type annotation

Private accessor members keep hash names and type annotations.

```ts:main.ts
class Foo {
  accessor #p: any;
}
```

```ts expected
class Foo {
    accessor #p: any;
}
```

### abstract accessor type annotation

Abstract accessor members keep type annotations.

```ts:main.ts
abstract class Foo {
  abstract accessor prop7: number;
}
```

```ts expected
abstract class Foo {
    abstract accessor prop7: number;
}
```

### accessor modifiers with decorators

Accessor members keep mixed modifiers and hash names.

```ts:main.ts
abstract class Foo {
  abstract accessor prop7: number;
  accessor #p: any;
  accessor a: any;
}
```

```ts expected
abstract class Foo {
    abstract accessor prop7: number;
    accessor #p: any;
    accessor a: any;
}
```
