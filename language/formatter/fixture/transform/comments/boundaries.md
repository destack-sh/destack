# Comment Boundaries

Comment boundary fixtures cover comments placed between neighboring syntax nodes.

## Calls and Members

### optional call with line boundary comment

Line comments before optional calls stay attached to the full call.

```tspp:main.tspp
const value = target // opt-call
?.()
```

```tspp expected
const value = target?.(); // opt-call
```

### optional call with block boundary comment

Block comments before optional calls stay attached at the call boundary.

```tspp:main.tspp
const value = target /* opt-call */ ?.()
```

```tspp expected
const value = target /* opt-call */?.();
```

### trailing comment on last call argument

Trailing comments on the last argument stay with that argument.

```tspp:main.tspp
call(
  value,
  other // last-arg
)
```

```tspp expected
call(
    value,
    other, // last-arg
);
```

### inline comment between member hops

Inline comments between member hops stay attached to the same hop.

```tspp:main.tspp
const value = source /* hop-a */ .first() /* hop-b */ .second()
```

```tspp expected
const value = source /* hop-a */
    .first() /* hop-b */
    .second();
```

## Statements

### if head trailing comment

Line comments on if heads stay with the condition line.

```tspp:main.tspp
if (ready) // if-head
    run()
```

```tspp expected
if (ready)
    // if-head
    run();
```

### else boundary comment

Boundary comments between if and else stay on the boundary.

```tspp:main.tspp
if (ready) {
  run()
}
// else-boundary
else {
  stop()
}
```

```tspp expected
if (ready) {
    run()
}
// else-boundary
else {
    stop()
}
```

### return trailing comment

Trailing comments on return expressions stay on the return line.

```tspp:main.tspp
function run() {
  return compute() // return-tail
}
```

```tspp expected
function run() {
    return compute(); // return-tail
}
```

## Control Flow

### control rvalue argument comments

Comments before, inside, and after control expressions in argument position keep their attachment.

```tspp
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

```tspp expected
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

```tspp
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

```tspp expected
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

```tspp:main.tspp
while (running) {
  if (done) break // break-tail
  tick()
}
```

```tspp expected
while (running) {
    if (done) break; // break-tail
    tick();
}
```

### continue trailing comment

Trailing comments on `continue` stay attached to the continue statement.

```tspp:main.tspp
for (const item of items) {
  if (!item) continue // continue-tail
  use(item)
}
```

```tspp expected
for (const item of items) {
    if (!item) continue; // continue-tail
    use(item);
}
```

### switch case boundary comments

Case boundary comments stay attached to the same case block.

```tspp:main.tspp
switch (state) {
  // before-ready
  case "ready":
    start() // ready-tail
    break
  default:
    stop() // default-tail
}
```

```tspp expected
switch (state) {
    // before-ready
    case "ready":
        start(); // ready-tail
        break;
    default:
        stop(); // default-tail
}
```

## Collections

### array element comments stay ordered

Comments around array elements keep source order.

```tspp:main.tspp
const list = [
  first, // first-tail
  /* second-head */ second,
  third // third-tail
]
```

```tspp expected
const list = [
    first, // first-tail
    /* second-head */ second,
    third, // third-tail
];
```

### object property trailing comments

Trailing property comments stay with the same property.

```tspp:main.tspp
const config = {
  first: 1, // first-tail
  second: 2 /* second-tail */
}
```

```tspp expected
const config = {
    first: 1, // first-tail
    second: 2 /* second-tail */,
};
```

## Types

### union line comment attachment

Line comments between union arms stay between the same arms.

```tspp:main.tspp line-width=24
type Value = First | // union-line
    Second | Third
```

```tspp expected
type Value =
    | First // union-line
    | Second
    | Third;
```

### union block comment attachment

Block comments between union arms stay between the same arms.

```tspp:main.tspp line-width=24
type Value = First /* union-block */ | Second
```

```tspp expected
type Value =
    | First /* union-block */
    | Second;
```

### intersection line comment attachment

Line comments between intersection members stay between the same members.

```tspp:main.tspp line-width=24
type Value = First & // intersection-line
    Second
```

```tspp expected
type Value = First & // intersection-line
    Second;
```

## Directives

### strict directive boundary comments

Comments around strict directives keep adjacency semantics.

```tspp:main.tspp
/******/ "use strict" /**/
/******/ run()
```

```tspp expected
/******/ "use strict"; /**/
/******/ run();
```

## Modules

### import specifier trailing comment

Specifier comments stay attached to the same import specifier.

```tspp:main.tspp
import { first, // first-spec
second } from "mod"
```

```tspp expected
import {
    first, // first-spec
    second,
} from "mod";
```

### export specifier trailing comment

Specifier comments stay attached to the same export specifier.

```tspp:main.tspp
export {
  first, // first-export
  second
}
```

```tspp expected
export {
    first, // first-export
    second,
};
```

## Other Boundaries

### try catch boundary comments

Boundary comments in try and catch blocks stay attached to the same statement line.

```tspp:main.tspp
try {
  run() // try-tail
} catch (error) {
  recover(error) // catch-tail
}
```

```tspp expected
try {
    run() // try-tail
} catch (error) {
    recover(error) // catch-tail
}
```

### yield boundary comments

Trailing comments on yield expressions stay attached to the yield expression line.

```tspp:main.tspp
function* run() {
  yield value // yield-tail
}
```

```tspp expected
function* run() {
    yield value; // yield-tail
}
```

### class property boundary comments

Trailing comments on class properties stay attached to the same property.

```tspp:main.tspp
class Box {
  first = 1 // first-tail
  second = 2 // second-tail
}
```

```tspp expected
class Box {
    first = 1; // first-tail
    second = 2; // second-tail
}
```

### assignment pattern boundary comments

Comments in assignment patterns stay attached to the same pattern segment.

```tspp:main.tspp
const {
  first = fallbackA, // assign-a
  second = fallbackB // assign-b
} = source
```

```tspp expected
const {
    first = fallbackA, // assign-a
    second = fallbackB, // assign-b
} = source;
```

Expression boundary fixtures cover comments inside expression formatting.

## Conditionals

### ternary branch inline comments

Inline comments in ternary branches stay attached to branch expressions.

```tspp:main.tspp line-width=40
const value = cond ? left /* left-note */ : right /* right-note */
```

```tspp expected
const value = cond
    ? left /* left-note */
    : right; /* right-note */
```

### ternary alternate line comment

Line comments in ternary alternates stay attached to alternates.

```tspp:main.tspp line-width=30
const value = cond ? left : // alt-line
right
```

```tspp expected
const value = cond
    ? left
    : // alt-line
      right;
```

### conditional argument trailing comment

Trailing comments on conditional arguments stay attached to the same argument.

```tspp:main.tspp
cb(
  overflowing ? "absolute top-0" : "relative", // conditional-tail
  parameter
)
```

```tspp expected
cb(
    overflowing ? "absolute top-0" : "relative", // conditional-tail
    parameter,
);
```

### conditional with new expression boundary comment

Boundary comments around `new` expressions stay attached inside conditional branches.

```tspp:main.tspp line-width=36
const value = cond ? new Left() /* left-new */ : new Right()
```

```tspp expected
const value = cond
    ? new Left() /* left-new */
    : new Right();
```

## Binary and Assignment

### binary operator boundary comment

Comments at binary operator boundaries stay with the same operator group.

```tspp:main.tspp line-width=40
const value = left + /* plus-note */ right + next
```

```tspp expected
const value = left
    + /* plus-note */ right
    + next;
```

### assignment right side comment

Comments after assignment right sides stay on assignment lines.

```tspp:main.tspp
value = compute() // assign-tail
```

```tspp expected
value = compute(); // assign-tail
```

## Calls

### first argument boundary comment

Boundary comments before first call arguments stay in call argument context.

```tspp:main.tspp
call(
  // first-arg
  first,
  second
)
```

```tspp expected
call(
    // first-arg
    first,
    second,
);
```

### call argument empty line preservation

Intentional empty lines between call arguments are preserved.

```tspp:main.tspp
call(
  first,

  second,
)
```

```tspp expected
call(
    first,

    second,
);
```

### call argument trailing inline block comment

Inline block comments after arguments stay attached to those arguments.

```tspp:main.tspp
call(
  first /* first-inline */,
  second,
)
```

```tspp expected
call(first /* first-inline */, second);
```

### callback argument trailing comment before comma

Trailing comments on callback arguments stay attached to that argument before the comma.

```tspp:main.tspp
call(
  () => {
    work()
  }, // callback-tail
  "good"
)
```

```tspp expected
call(
    () => {
        work()
    }, // callback-tail
    "good",
);
```

## Chains

### optional chain boundary comment

Comments before optional chain operators stay on the preceding segment.

```tspp:main.tspp
const value = source
  .first /* first-boundary */
  ?.second()
```

```tspp expected
const value = source.first /* first-boundary */
    ?.second();
```

### computed member boundary comment

Comments before computed members stay attached to the preceding segment.

```tspp:main.tspp
const value = source /* before-index */ [key]
```

```tspp expected
const value = source /* before-index */[key];
```

## New and Parentheses

### new expression callee boundary comment

Comments between `new` callee and arguments stay at the call boundary.

```tspp:main.tspp
const value = new Factory /* new-call */ (arg)
```

```tspp expected
const value = new Factory(/* new-call */ arg);
```

## Template Literals and Unary Boundaries

### tagged template with trailing call comment

Trailing comments near tagged template expressions stay attached to template expression lines.

```tspp:main.tspp
const value = css`color: red;` // css-tail
```

```tspp expected
const value = css`color: red;`; // css-tail
```

### template member expression interpolation comment

Comments inside template interpolations stay attached to interpolation expressions.

```tspp:main.tspp
const value = `${source /* member-note */ .name}`
```

```tspp expected
const value = `${source /* member-note */.name}`;
```

### unary negative numeric comment boundary

Comments after unary minus stay attached to the unary expression.

```tspp:main.tspp
const value = -/* unary-note */ 1
```

```tspp expected
const value = -(/* unary-note */ 1);
```

### unary negative line comment boundary

Line comments after unary minus stay attached to the unary expression with stable multiline wrapping.

```tspp:main.tspp
const value = -// unary-line-note
1
```

```tspp expected
const value = -(
    // unary-line-note
    1
);
```

### label expression boundary comment

Label comments stay attached to the labeled statement body.

```tspp:main.tspp
start: // label-tail
while (true) {
  break start
}
```

```tspp expected
// label-tail
start: while (true) {
    break start;
}
```

## Callee and First Argument Comment Shapes

### callee boundary comment before call parentheses

Callee boundary comments stay attached before call parentheses.

```tspp:main.tspp
const value = run /* callee-note */ (first, second)
```

```tspp expected
const value = run(/* callee-note */ first, second);
```

### first argument expansion with leading comment

Leading first argument comments stay in first argument positions during expansion.

```tspp:main.tspp line-width=34
const value = compute(
  // first-note
  veryLongFirstArgument,
  second,
)
```

```tspp expected
const value = compute(
    // first-note
    veryLongFirstArgument,
    second,
);
```

### tree first argument boundary comment

Tree first-argument comments stay attached to the same tree argument line.

```tspp:main.tspp
const value = compute(
  <Card />, // first-tree
  second,
)
```

```tspp expected
const value = compute(
    <Card />, // first-tree
    second,
);
```

## Control Comment Permutations

### if condition same line trailing comment

If condition trailing comments stay attached to the condition line.

```tspp:main.tspp
if (ready && enabled) // if-condition
  run()
```

```tspp expected
if (ready && enabled)
    // if-condition
    run();
```

### return statement nested call comment

Return statement trailing comments stay on return lines with nested calls.

```tspp:main.tspp
function run() {
  return compute(value) // return-note
}
```

```tspp expected
function run() {
    return compute(value); // return-note
}
```

### try catch expression comments

Try and catch expression comments stay attached to expression lines.

```tspp:main.tspp
try {
  run(value) // try-note
} catch (error) {
  recover(error) // catch-note
}
```

```tspp expected
try {
    run(value) // try-note
} catch (error) {
    recover(error) // catch-note
}
```
