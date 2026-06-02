# Expression Annotation Boundaries

Expression annotation fixtures cover assertions, decorators, and boundary comments.

## Assertions and Satisfies

### as assertion with trailing line comment

Line comments after `as` assertions stay attached to the same assertion.

```ts:main.ts
const value = source as // as-tail
number
```

```ts expected
const value = source as number; // as-tail
```

### nested await with as assertion

Nested `await` and `as` assertions keep stable grouping and attachment.

```ts:main.ts
const value = (await load()) as Promise<Result<string, Error>>
```

```ts expected
const value = (await load()) as Promise<Result<string, Error>>;
```

### const assertion with boundary comment

Boundary comments next to `as const` stay attached to the asserted expression.

```ts:main.ts
const values = [1, 2, 3] /* as-const */ as const
```

```ts expected
const values = [1, 2, 3] /* as-const */ as const;
```

### satisfies with trailing line comment

Line comments around `satisfies` stay attached to the satisfies clause.

```ts:main.ts line-width=50
const config = { retries: 3 } satisfies // sat-tail
Record<string, number>
```

```ts expected
const config = { retries: 3 } satisfies Record< // sat-tail
    string,
    number
>;
```

## TypeScript Assertions and Satisfies Comments

### as const with inline comment after const

Inline comments after `as const` stay on the same assertion line.

```ts:main.ts
1 as const // const-tail
;
```

```ts expected
1 as const; // const-tail
```

### as const with line comment before const

Line comments between `as` and `const` stay attached to the assertion target.

```ts:main.ts
1 as // before-const
const;
```

```ts expected
1 as const; // before-const
```

### block comment between as and type target

Block comments between `as` and target types stay attached to the assertion.

```ts:main.ts
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
```

```ts expected
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
```

### assertion chain with trailing block comment

Trailing block comments on assertion chains stay attached before the following member call.

```ts:main.ts indent-width=2 line-width=80
(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
```

```ts expected
(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
```

### multiline postfix comment before const

Multiline block comments before `const` stay attached to the assertion boundary.

```ts:main.ts indent-width=2 line-width=80
{
1 as /*
comment
*/const;
}
```

```ts expected
{
  1 as const /*
comment
*/;
}
```

### assertion and satisfies boundary comments

Block comments between `as` and `satisfies` targets stay attached to the same operator boundary.

```ts:main.ts indent-width=2 line-width=80
{
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
}
```

```ts expected
{
  1 as /* between */ Foo;
  1 satisfies /* sat-between */ Foo;
}
```

### multiline block comment between as and const

Multiline block comments between `as` and `const` stay attached to `const` assertions.

```ts:main.ts
1 as /*
block-comment
*/ const;
```

```ts expected
1 as const /*
block-comment
*/;
```

### nested await chain with as assertion

Nested await chains with `as` assertions keep stable grouping.

```ts:main.ts
const count = (await
  ((await (
    await focusOnSection("bookmarks")
  ).findItem("mine")) as TreeItem
).getChildren()
).length
```

```ts expected
const count = (
    await ((await (await focusOnSection("bookmarks")).findItem("mine")) as TreeItem).getChildren()
).length;
```

### member assignment through as assertion target

Assignments through parenthesized `as` assertion targets preserve assignment shape.

```ts:main.ts
(foo.bar as Baz) = value
(foo.bar as any)++
```

```ts expected
(foo.bar as Baz) = value;
(foo.bar as any)++;
```

## Decorated Class Expressions

### decorated class expression with member access

Decorated class expressions keep stable wrapping before member access.

```js:main.js
(@deco
class Foo {}).name
```

```js expected
(
    @deco
    class Foo {}
).name;
```

### decorated anonymous class expression with member access

Decorated anonymous class expressions keep stable wrapping before member access.

```js:main.js
(@deco
class {}).name
```

```js expected
(
    @deco
    class {}
).name;
```

## Decorated Class Expressions

### decorated class expression in argument position

Decorated class expressions in argument positions keep stable wrapping and attachment.

```js:main.js
use((@decorator class {}))
```

```js expected
use(
    @decorator
    class {},
);
```

## Satisfies Operator Boundary Permutations

### satisfies expression statement with trailing comment

Expression statement `satisfies` comments stay attached to the satisfies boundary.

```ts:main.ts
({ value: 1 } satisfies Record<string, number>) // sat-expression
```

```ts expected
({ value: 1 }) satisfies Record<string, number>; // sat-expression
```

### nested await with satisfies and boundary comment

Nested await chains with `satisfies` keep stable grouping and boundary comments.

```ts:main.ts
const value = (await load()) satisfies // sat-await
Promise<Result<string, Error>>
```

```ts expected
const value = (await load()) satisfies Promise<Result<string, Error>>; // sat-await
```
