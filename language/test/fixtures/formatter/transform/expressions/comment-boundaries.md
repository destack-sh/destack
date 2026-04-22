# Expression Comment Boundaries

Tests for expression level comment ownership and stability.

## Conditionals

### ternary branch inline comments

Inline comments in ternary branches stay attached to branch expressions.

```ts:main.ts line-width=40
const value = cond ? left /* left-note */ : right /* right-note */
```

```ts expected
const value = cond
    ? left /* left-note */
    : right; /* right-note */
```

### ternary alternate line comment

Line comments in ternary alternates stay attached to alternates.

```ts:main.ts line-width=30
const value = cond ? left : // alt-line
right
```

```ts expected
const value = cond
    ? left // alt-line
    : right;
```

### conditional argument trailing comment

Trailing comments on conditional arguments stay attached to the same argument.

```ts:main.ts
cb(
  overflowing ? "absolute top-0" : "relative", // conditional-tail
  parameter
)
```

```ts expected
cb(
    overflowing ? "absolute top-0" : "relative", // conditional-tail
    parameter,
);
```

### conditional with new expression boundary comment

Boundary comments around `new` expressions stay attached inside conditional branches.

```ts:main.ts line-width=36
const value = cond ? new Left() /* left-new */ : new Right()
```

```ts expected
const value = cond
    ? new Left() /* left-new */
    : new Right();
```

## Binary And Assignment

### binary operator boundary comment

Comments at binary operator boundaries stay with the same operator group.

```ts:main.ts line-width=40
const value = left + /* plus-note */ right + next
```

```ts expected
const value =
    left + /* plus-note */ right + next;
```

### assignment right side comment

Comments after assignment right sides stay on assignment lines.

```ts:main.ts
value = compute() // assign-tail
```

```ts expected
value = compute(); // assign-tail
```

## Calls

### first argument boundary comment

Boundary comments before first call arguments stay in call argument context.

```ts:main.ts
call(
  // first-arg
  first,
  second
)
```

```ts expected
call(
    // first-arg
    first,
    second,
);
```

### call argument empty line preservation

Intentional empty lines between call arguments are preserved.

```ts:main.ts
call(
  first,

  second,
)
```

```ts expected
call(
    first,

    second,
);
```

### call argument trailing inline block comment

Inline block comments after arguments stay attached to those arguments.

```ts:main.ts
call(
  first /* first-inline */,
  second,
)
```

```ts expected
call(first /* first-inline */, second);
```

### callback argument trailing comment before comma

Trailing comments on callback arguments stay attached to that argument before the comma.

```ts:main.ts
call(
  () => {
    work()
  }, // callback-tail
  "good"
)
```

```ts expected
call(
    () => {
        work();
    }, // callback-tail
    "good",
);
```

## Chains

### optional chain boundary comment

Comments before optional chain operators stay on the preceding segment.

```ts:main.ts
const value = source
  .first /* first-boundary */
  ?.second()
```

```ts expected
const value = source.first /* first-boundary */
    ?.second();
```

### computed member boundary comment

Comments before computed members stay attached to the preceding segment.

```ts:main.ts
const value = source /* before-index */ [key]
```

```ts expected
const value = source /* before-index */[key];
```

## New And Parentheses

### new expression callee boundary comment

Comments between `new` callee and arguments stay at the call boundary.

```ts:main.ts
const value = new Factory /* new-call */ (arg)
```

```ts expected
const value = new Factory(/* new-call */ arg);
```

## Template Literals And Unary Boundaries

### tagged template with trailing call comment

Trailing comments near tagged template expressions stay attached to template expression lines.

```ts:main.ts
const value = css`color: red;` // css-tail
```

```ts expected
const value = css`color: red;`; // css-tail
```

### template member expression interpolation comment

Comments inside template interpolations stay attached to interpolation expressions.

```ts:main.ts
const value = `${source /* member-note */ .name}`
```

```ts expected
const value = `${source /* member-note */.name}`;
```

### unary negative numeric comment boundary

Comments after unary minus stay attached to the unary expression.

```ts:main.ts
const value = -/* unary-note */ 1
```

```ts expected
const value = -(/* unary-note */ 1);
```

### unary negative line comment boundary

Line comments after unary minus stay attached to the unary expression with stable multiline layout.

```ts:main.ts
const value = -// unary-line-note
1
```

```ts expected
const value = -(
    // unary-line-note
    1
);
```

### label expression boundary comment

Label comments stay attached to the labeled statement body.

```ts:main.ts
start: // label-tail
while (true) {
  break start
}
```

```ts expected
// label-tail
start: while (true) {
    break start;
}
```

## Callee And First Argument Comment Shapes

### callee boundary comment before call parentheses

Callee boundary comments stay attached before call parentheses.

```ts:main.ts
const value = run /* callee-note */ (first, second)
```

```ts expected
const value = run(/* callee-note */ first, second);
```

### first argument expansion with leading comment

Leading first argument comments stay in first argument positions during expansion.

```ts:main.ts line-width=34
const value = compute(
  // first-note
  veryLongFirstArgument,
  second,
)
```

```ts expected
const value = compute(
    // first-note
    veryLongFirstArgument,
    second,
);
```

### jsx first argument boundary comment

JSX first argument comments stay attached to the same JSX argument line.

```tsx:main.tsx
const value = compute(
  <Card />, // first-jsx
  second,
)
```

```tsx expected
const value = compute(
    <Card />, // first-jsx
    second,
);
```

## Control Comment Permutations

### if condition same line trailing comment

If condition trailing comments stay attached to the condition line.

```ts:main.ts
if (ready && enabled) // if-condition
  run()
```

```ts expected
if (ready && enabled)
    // if-condition
    run();
```

### return statement nested call comment

Return statement trailing comments stay on return lines with nested calls.

```ts:main.ts
function run() {
  return compute(value) // return-note
}
```

```ts expected
function run() {
    return compute(value); // return-note
}
```

### try catch expression comments

Try and catch expression comments stay attached to expression lines.

```ts:main.ts
try {
  run(value) // try-note
} catch (error) {
  recover(error) // catch-note
}
```

```ts expected
try {
    run(value); // try-note
} catch (error) {
    recover(error); // catch-note
}
```
