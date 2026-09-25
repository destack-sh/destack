# Expression Annotation Boundaries

Expression annotation fixtures cover assertions, decorators, and boundary comments.

## Assertions and Satisfies

### as assertion with trailing line comment

Line comments after `as` assertions stay attached to the same assertion.

```tspp:main.tspp
const value = source as // as-tail
number
```

```tspp expected
const value = source as number; // as-tail
```

### nested await with as assertion

Nested `await` and `as` assertions keep stable grouping and attachment.

```tspp:main.tspp
const value = (await load()) as Promise<Result<string, Error>>
```

```tspp expected
const value = (await load()) as Promise<Result<string, Error>>;
```

### const assertion with boundary comment

Boundary comments next to `as const` stay attached to the asserted expression.

```tspp:main.tspp
const values = [1, 2, 3] /* as-const */ as const
```

```tspp expected
const values = [1, 2, 3] /* as-const */ as const;
```

### satisfies with trailing line comment

Line comments around `satisfies` stay attached to the satisfies clause.

```tspp:main.tspp line-width=50
const config = { retries: 3 } satisfies // sat-tail
Record<string, number>
```

```tspp expected
const config = { retries: 3 } satisfies Record< // sat-tail
    string,
    number
>;
```

## Assertions and Satisfies Comments

### as const with inline comment after const

Inline comments after `as const` stay on the same assertion line.

```tspp:main.tspp
1 as const // const-tail
;
```

```tspp expected
1 as const; // const-tail
```

### as const with line comment before const

Line comments between `as` and `const` stay attached to the assertion target.

```tspp:main.tspp
1 as // before-const
const;
```

```tspp expected
1 as const; // before-const
```

### block comment between as and type target

Block comments between `as` and target types stay attached to the assertion.

```tspp:main.tspp
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
```

```tspp expected
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
```

### assertion chain with trailing block comment

Trailing block comments on assertion chains stay attached before the following member call.

```tspp:main.tspp indent-width=2 line-width=80
(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
```

```tspp expected
(activeService as unknown as QuickInputController) /* TS fail */
  .pick();
```

### multiline postfix comment before const

Multiline block comments before `const` stay attached to the assertion boundary.

```tspp:main.tspp indent-width=2 line-width=80
{
1 as /*
comment
*/const;
}
```

```tspp expected
{
  1 as const /*
comment
*/;
}
```

### assertion and satisfies boundary comments

Block comments between `as` and `satisfies` targets stay attached to the same operator boundary.

```tspp:main.tspp indent-width=2 line-width=80
{
1 as /* between */ Foo;
1 satisfies /* sat-between */ Foo;
}
```

```tspp expected
{
  1 as /* between */ Foo;
  1 satisfies /* sat-between */ Foo;
}
```

### multiline block comment between as and const

Multiline block comments between `as` and `const` stay attached to `const` assertions.

```tspp:main.tspp
1 as /*
block-comment
*/ const;
```

```tspp expected
1 as const /*
block-comment
*/;
```

### nested await chain with as assertion

Nested await chains with `as` assertions keep stable grouping.

```tspp:main.tspp
const count = (await
  ((await (
    await focusOnSection("bookmarks")
  ).findItem("mine")) as TreeItem
).getChildren()
).length
```

```tspp expected
const count = (
    await ((await (await focusOnSection("bookmarks")).findItem("mine")) as TreeItem).getChildren()
).length;
```

### member assignment through as assertion target

Assignments through parenthesized `as` assertion targets preserve assignment shape.

```tspp:main.tspp
(foo.bar as Baz) = value
(foo.bar as any)++
```

```tspp expected
(foo.bar as Baz) = value;
(foo.bar as any)++;
```

## Decorated Class Expressions

### decorated class expression with member access

Decorated class expressions keep stable wrapping before member access.

```tspp:main.tspp
(@deco
class Foo {}).name
```

```tspp expected
(
    @deco
    class Foo {}
).name;
```

### decorated anonymous class expression with member access

Decorated anonymous class expressions keep stable wrapping before member access.

```tspp:main.tspp
(@deco
class {}).name
```

```tspp expected
(
    @deco
    class {}
).name;
```

## Decorated Class Expressions

### decorated class expression in argument position

Decorated class expressions in argument positions keep stable wrapping and attachment.

```tspp:main.tspp
use((@decorator class {}))
```

```tspp expected
use(
    @decorator
    class {},
);
```

## Satisfies Operator Boundary Permutations

### satisfies expression statement with trailing comment

Expression statement `satisfies` comments stay attached to the satisfies boundary.

```tspp:main.tspp
({ value: 1 } satisfies Record<string, number>) // sat-expression
```

```tspp expected
({ value: 1 }) satisfies Record<string, number>; // sat-expression
```

### nested await with satisfies and boundary comment

Nested await chains with `satisfies` keep stable grouping and boundary comments.

```tspp:main.tspp
const value = (await load()) satisfies // sat-await
Promise<Result<string, Error>>
```

```tspp expected
const value = (await load()) satisfies Promise<Result<string, Error>>; // sat-await
```
