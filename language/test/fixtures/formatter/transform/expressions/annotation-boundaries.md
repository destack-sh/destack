# Expression Annotation Boundaries

Tests for annotation and decorator attachment stability in expression contexts.

## Closure Type Cast Comments

### closure type cast call expression

Closure type cast comments stay attached inside the cast parentheses.

```js:main.js
let assignment = (/** @type {string} */ getValue())
```

```js expected
let assignment = /** @type {string} */ getValue();
```

### closure type cast member base expression

Type cast comments on member bases stay attached to the base expression.

```js:main.js
var newArray = (/** @type {array} */ numberOrString).map((x) => x)
```

```js expected
var newArray = /** @type {array} */ numberOrString.map((x) => x);
```

### closure type cast with neighboring block comment

Neighboring block comments keep their relative order with type cast comments.

```js:main.js
(/* 2 */ /** @type {{bar: string[]}} */ {}).bar.forEach(doStuff)
(/** @type {{bar: string[]}} */ /* 2 */ {}).bar.forEach(doStuff)
```

```js expected
/* 2 */ /** @type {{bar: string[]}} */ ({}).bar
    .forEach(doStuff)(/** @type {{bar: string[]}} */ /* 2 */ {})
    .bar.forEach(doStuff);
```

## TypeScript Assertions And Satisfies

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

## Closure Type Cast Shapes

### closure cast in no-semi prefix statement

Leading semicolon no-semi statements keep closure cast comments attached.

```js:main.js
;/* keep-2 */ /** @type {{bar: string[]}} */ ({}).bar.forEach(doStuff)
;/** @type {{bar: string[]}} */ /* keep-2 */ ({}).bar.forEach(doStuff)
```

```js expected
/* keep-2 */ /** @type {{bar: string[]}} */ ({}).bar.forEach(doStuff);
/** @type {{bar: string[]}} */ /* keep-2 */ ({}).bar.forEach(doStuff);
```

### closure cast in binary expression

Closure cast comments in binary expressions stay attached to cast operands.

```js:main.js
test((/** @type {number} */ num) + 1)
test((/** @type {!Array} */ arrOrString).length + 1)
```

```js expected
test(/** @type {number} */ num + 1);
test(/** @type {!Array} */ arrOrString.length + 1);
```

### closure cast in function argument

Closure cast comments in call arguments stay attached to argument expressions.

```js:main.js
const data = functionCall(
  arg1,
  arg2,
  /** @type {{height: number, width: number}} */ (arg3),
)
```

```js expected
const data = functionCall(arg1, arg2, /** @type {{height: number, width: number}} */ (arg3));
```

## TypeScript Assertions And Satisfies Comments

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
const count = (await ((await (
    await focusOnSection("bookmarks")
).findItem("mine")) as TreeItem).getChildren()).length;
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


## Closure Type Cast Advanced Contexts

### closure cast in object and array literals

Closure type cast comments stay attached in object and array literal slots.

```js:main.js
const value = {
  item: /** @type {!Array<string>} */ (input),
  list: [/** @type {!Array<number>} */ (numbers)],
}
```

```js expected
const value = {
    item: /** @type {!Array<string>} */ (input),
    list: [/** @type {!Array<number>} */ (numbers)],
};
```

### closure cast in nested chains

Nested closure type cast comments stay attached through member and call chains.

```js:main.js
const value = (/** @type {{inner: {run: () => number}}} */ (source)).inner.run()
```

```js expected
const value = /** @type {{inner: {run: () => number}}} */ (source).inner.run();
```

### closure cast in class heritage

Closure type cast comments in class heritage stay attached to the superclass expression.

```js:main.js
class Box extends /** @type {{new (): Base}} */ (baseFactory()) {}
```

```js expected
class Box extends /** @type {{new (): Base}} */ (baseFactory()) {}
```

### closure cast in no-semi multiline parenthesized call

No-semi multiline starts keep closure type cast comments attached and ordered.

```js:main.js
;(
  /** @type {{run: () => void}} */ (factory())
).run()
```

```js expected
/** @type {{run: () => void}} */ (factory()).run();
```

### closure cast with satisfies type boundary

Closure type cast comments keep stable attachment near `satisfies` boundaries.

```ts:main.ts
const value = /** @type {{ok: boolean}} */ ({ ok: true }) satisfies Record<string, unknown>
```

```ts expected
const value = /** @type {{ok: boolean}} */ ({ ok: true }) satisfies Record<string, unknown>;
```

## Closure Type Cast Conformance Permutations

### closure cast with no-semi neighboring comments

No-semi closure casts keep neighboring comments attached and ordered.

```js:main.js
;/** @type {{bar: string[]}} */ ({}).bar // bar-tail
.forEach(doStuff)
```

```js expected
/** @type {{bar: string[]}} */ ({}).bar // bar-tail
    .forEach(doStuff);
```

### closure cast non-cast parentheses stay ordinary

Parenthesized expressions without cast comments stay ordinary expressions.

```js:main.js
const value = (/* ordinary */ source).next()
```

```js expected
const value = /* ordinary */ source.next();
```

### closure cast in first argument expansion path

Closure casts in first argument expansion paths keep cast attachment to argument expressions.

```js:main.js
target(
  /** @type {{id: string}} */ (entry),
  second,
)
```

```js expected
target(/** @type {{id: string}} */ (entry), second);
```

### closure cast in rest element comment path

Closure casts near rest element comments keep attachment during rest parsing.

```js:main.js
function run(.../* rest-head */ args) {
  return /** @type {!Array<string>} */ (args)
}
```

```js expected
function run(.../* rest-head */ args) {
    return /** @type {!Array<string>} */ (args);
}
```

## Decorated Class Expression Contexts

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

### decorated class expression in extends position

Decorated class expressions in extends positions keep stable attachment.

```js:main.js
class Derived extends (@decorator class Base {}) {}
```

```js expected
class Derived extends (
    @decorator
    class Base {}
) {}
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

Nested await chains with satisfies keep stable grouping and boundary comments.

```ts:main.ts
const value = (await load()) satisfies // sat-await
Promise<Result<string, Error>>
```

```ts expected
const value = (await load()) satisfies Promise<Result<string, Error>>; // sat-await
```
