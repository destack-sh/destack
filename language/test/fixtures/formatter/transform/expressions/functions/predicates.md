# Type Predicates

## Type Predicates

### type predicate return type

Type predicate return types keep `is` spacing.

```ts:main.ts
function isFoo(value: unknown): value is Foo { return value instanceof Foo }
```

```ts expected
function isFoo(value: unknown): value is Foo {
    return value instanceof Foo;
}
```

### type predicate with contextual type subject

Type predicate subjects can use names that are contextual type literals elsewhere.

```ts:main.d.ts
declare function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike
```

```ts expected
declare function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike;
```

### asserts type predicate return type

Asserted type predicates keep `asserts` and `is` spacing.

```ts:main.ts
function assertFoo(value: Foo): asserts value is Foo { return value !== null }
```

```ts expected
function assertFoo(value: Foo): asserts value is Foo {
    return value !== null;
}
```

### asserts type predicate comments

Comments before and after `is` stay inside the asserted predicate.

```ds
function assertFoo(value: unknown): asserts value /* value */ is /* type */ Foo { return }
```

```ds expected
function assertFoo(value: unknown): asserts value /* value */ is /* type */ Foo {
    return;
}
```

### asserts subject without predicate

Asserted subjects without predicates keep the `asserts` keyword.

```ts:main.ts
function assertDefined(value: Foo | null): asserts value { if (value === null) throw new Error() }
```

```ts expected
function assertDefined(value: Foo | null): asserts value {
    if (value === null) throw new Error();
}
```
