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
