# Annotation Syntax

## Annotation Syntax Edges

### private accessor hash type annotation

Accessor hash members with type annotations are parsed and formatted.

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

### abstract declare accessor type annotation

Abstract accessor fields with type annotations are parsed and formatted.

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

### decorator class expression as superclass

Decorated class expressions are parsed in superclass positions.

```js:main.js
class Outer extends
  @deco
  class {} {}
```

```js expected
class Outer extends (
    @deco
    class {}
) {}
```

### TypeScript accessor modifiers with decorators

Accessor fields with mixed modifiers and hash members are parsed and formatted.

```ts:main.ts
abstract class Foo {
  abstract accessor prop7: number;
  accessor #p: any;
  accessor a!: any;
}
```

```ts expected
abstract class Foo {
    abstract accessor prop7: number;
    accessor #p: any;
    accessor a!: any;
}
```
