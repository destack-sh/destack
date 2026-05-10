# Comment Boundaries

Comment boundary fixtures cover comments placed between neighboring syntax nodes.

## Calls and Members

### optional call with line boundary comment

Line comments before optional calls stay attached to the full call.

```ts:main.ts
const value = target // opt-call
?.()
```

```ts expected
const value = target?.(); // opt-call
```

### optional call with block boundary comment

Block comments before optional calls stay attached at the call boundary.

```ts:main.ts
const value = target /* opt-call */ ?.()
```

```ts expected
const value = target /* opt-call */?.();
```

### trailing comment on last call argument

Trailing comments on the last argument stay with that argument.

```ts:main.ts
call(
  value,
  other // last-arg
)
```

```ts expected
call(
    value,
    other, // last-arg
);
```

### inline comment between member hops

Inline comments between member hops stay attached to the same hop.

```ts:main.ts
const value = source /* hop-a */ .first() /* hop-b */ .second()
```

```ts expected
const value = source /* hop-a */
    .first() /* hop-b */
    .second();
```

## Statements

### if head trailing comment

Line comments on if heads stay with the condition line.

```ts:main.ts
if (ready) // if-head
    run()
```

```ts expected
if (ready)
    // if-head
    run();
```

### else boundary comment

Boundary comments between if and else stay on the boundary.

```ts:main.ts
if (ready) {
  run()
}
// else-boundary
else {
  stop()
}
```

```ts expected
if (ready) {
    run();
}
// else-boundary
else {
    stop();
}
```

### return trailing comment

Trailing comments on return expressions stay on the return line.

```ts:main.ts
function run() {
  return compute() // return-tail
}
```

```ts expected
function run() {
    return compute(); // return-tail
}
```

### throw trailing comment

Trailing comments on throw expressions stay on the throw line.

```ts:main.ts
function fail() {
  throw error // throw-tail
}
```

```ts expected
function fail() {
    throw error; // throw-tail
}
```

## Control Flow

### control rvalue argument comments

Comments before, inside, and after control expressions in argument position keep their attachment.

```ds
render(
    // state
    if (ready) {
        // ready value
        buildReady(context)
    } else {
        // pending value
        buildPending(context)
    }, // state tail
    match (kind) {
        // primary
        Primary => buildPrimary(context)
        // fallback
        _ => buildFallback(context)
    }
)
```

```ds expected
render(
    // state
    if (ready) {
        // ready value
        buildReady(context)
    } else {
        // pending value
        buildPending(context)
    }, // state tail
    match (kind) {
        // primary
        Primary => buildPrimary(context)
        // fallback
        _ => buildFallback(context)
    },
);
```

### control rvalue collection comments

Comments around control expressions in collection slots keep their attachment.

```ds
const values = [
    // selected
    if (ready) {
        // ready branch
        readyValue
    } else {
        // pending branch
        pendingValue
    },
    // mapped
    match (kind) {
        // primary
        Primary => primaryValue
        // fallback
        _ => fallbackValue
    }
]
```

```ds expected
const values = [
    // selected
    if (ready) {
        // ready branch
        readyValue
    } else {
        // pending branch
        pendingValue
    },
    // mapped
    match (kind) {
        // primary
        Primary => primaryValue
        // fallback
        _ => fallbackValue
    },
];
```

### break trailing comment

Trailing comments on `break` stay attached to the break statement.

```ts:main.ts
while (running) {
  if (done) break // break-tail
  tick()
}
```

```ts expected
while (running) {
    if (done) break; // break-tail
    tick();
}
```

### continue trailing comment

Trailing comments on `continue` stay attached to the continue statement.

```ts:main.ts
for (const item of items) {
  if (!item) continue // continue-tail
  use(item)
}
```

```ts expected
for (const item of items) {
    if (!item) continue; // continue-tail
    use(item);
}
```

### switch case boundary comments

Case boundary comments stay attached to the same case block.

```ts:main.ts
switch (state) {
  // before-ready
  case "ready":
    start() // ready-tail
    break
  default:
    stop() // default-tail
}
```

```ts expected
switch (state) {
    // before-ready
    case "ready":
        start(); // ready-tail
        break;
    default:
        stop(); // default-tail
}
```

### empty statement trailing comment

Trailing comments after empty statements stay attached to that statement.

```ts:main.ts
if (ready) ; // empty-tail
run()
```

```ts expected
if (ready); // empty-tail
run();
```

## Collections

### array element comments stay ordered

Comments around array elements keep source order.

```ts:main.ts
const list = [
  first, // first-tail
  /* second-head */ second,
  third // third-tail
]
```

```ts expected
const list = [
    first, // first-tail
    /* second-head */ second,
    third, // third-tail
];
```

### object property trailing comments

Trailing property comments stay with the same property.

```ts:main.ts
const config = {
  first: 1, // first-tail
  second: 2 /* second-tail */
}
```

```ts expected
const config = {
    first: 1, // first-tail
    second: 2 /* second-tail */,
};
```

## Types

### union line comment attachment

Line comments between union arms stay between the same arms.

```ts:main.ts line-width=24
type Value = First | // union-line
    Second | Third
```

```ts expected
type Value =
    | First // union-line
    | Second
    | Third;
```

### union block comment attachment

Block comments between union arms stay between the same arms.

```ts:main.ts line-width=24
type Value = First /* union-block */ | Second
```

```ts expected
type Value =
    | First /* union-block */
    | Second;
```

### intersection line comment attachment

Line comments between intersection members stay between the same members.

```ts:main.ts line-width=24
type Value = First & // intersection-line
    Second
```

```ts expected
type Value = First & // intersection-line
    Second;
```

## Directives

### strict directive boundary comments

Comments around strict directives keep adjacency semantics.

```ts:main.ts
/******/ "use strict" /**/
/******/ run()
```

```ts expected
/******/ "use strict"; /**/
/******/ run();
```

## Modules

### import specifier trailing comment

Specifier comments stay attached to the same import specifier.

```ts:main.ts
import { first, // first-spec
second } from "mod"
```

```ts expected
import {
    first, // first-spec
    second,
} from "mod";
```

### export specifier trailing comment

Specifier comments stay attached to the same export specifier.

```ts:main.ts
export {
  first, // first-export
  second
}
```

```ts expected
export {
    first, // first-export
    second,
};
```

## Other Boundaries

### try catch boundary comments

Boundary comments in try and catch blocks stay attached to the same statement line.

```ts:main.ts
try {
  run() // try-tail
} catch (error) {
  recover(error) // catch-tail
}
```

```ts expected
try {
    run(); // try-tail
} catch (error) {
    recover(error); // catch-tail
}
```

### yield boundary comments

Trailing comments on yield expressions stay attached to the yield expression line.

```ts:main.ts
function* run() {
  yield value // yield-tail
}
```

```ts expected
function* run() {
    yield value; // yield-tail
}
```

### class property boundary comments

Trailing comments on class properties stay attached to the same property.

```ts:main.ts
class Box {
  first = 1 // first-tail
  second = 2 // second-tail
}
```

```ts expected
class Box {
    first = 1; // first-tail
    second = 2; // second-tail
}
```

### assignment pattern boundary comments

Comments in assignment patterns stay attached to the same pattern segment.

```ts:main.ts
const {
  first = fallbackA, // assign-a
  second = fallbackB // assign-b
} = source
```

```ts expected
const {
    first = fallbackA, // assign-a
    second = fallbackB, // assign-b
} = source;
```

Expression boundary fixtures cover comments inside expression formatting.

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

## Binary and Assignment

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

## New and Parentheses

### new expression callee boundary comment

Comments between `new` callee and arguments stay at the call boundary.

```ts:main.ts
const value = new Factory /* new-call */ (arg)
```

```ts expected
const value = new Factory(/* new-call */ arg);
```

## Template Literals and Unary Boundaries

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

Line comments after unary minus stay attached to the unary expression with stable multiline wrapping.

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

## Callee and First Argument Comment Shapes

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
