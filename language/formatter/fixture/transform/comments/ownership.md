# Comment Ownership

Comment ownership fixtures cover attachment decisions across formatter boundaries.

## Optional Chain Boundaries

### block comment before optional call

Block comments before optional calls stay attached to the callee boundary.

```tspp:main.tspp
const value = call /* keep-call */ ?.()
```

```tspp expected
const value = call /* keep-call */?.();
```

### line comment before optional call

Line comments before optional calls stay attached to the full expression.

```tspp:main.tspp
const value = call // keep-line
?.()
```

```tspp expected
const value = call?.(); // keep-line
```

## Statement Boundaries

### if trailing condition comment

Line comments after conditions stay attached to the condition line.

```tspp:main.tspp
if (ready) // keep-condition
    run()
```

```tspp expected
if (ready)
    // keep-condition
    run();
```

### class field trailing block comment

Trailing block comments on class fields stay with the field declaration.

```tspp:main.tspp
class Box {
    value = 1 /* keep-field */
    next = 2
}
```

```tspp expected
class Box {
    value = 1; /* keep-field */
    next = 2;
}
```

## Type Boundaries

### union arm line comment

Comments between union arms are preserved between the same arms.

```tspp:main.tspp line-width=20
type Value = string | // keep-union
    number
```

```tspp expected
type Value =
    | string // keep-union
    | number;
```

### leading pipe union line comment

Leading-pipe union comments stay on the same declaration value.

```tspp:main.tspp line-width=20
type A2 =
  | A
  | B

type A3 =
  | // keep-leading-union
  C
  |
  D;
```

```tspp expected
type A2 = A | B;

type A3 =
    | // keep-leading-union
      C
    | D;
```

### union arm block comment

Block comments between union arms are preserved in place.

```tspp:main.tspp line-width=20
type Value = string /* keep-union */ | number
```

```tspp expected
type Value =
    | string /* keep-union */
    | number;
```

### union arm doc block comment

Doc block comments in union expressions are preserved on the same type side.

```tspp:main.tspp line-width=80
export type Value = /** keep-doc
 */
| { ok: true }
| { ok: false; value: bigint | null };
```

```tspp expected
export type Value =
    /// keep-doc
    { ok: true } | { ok: false; value: bigint | null };
```

## Variable Declarations

### variable trailing marker line comment

Marker comments at declaration tails are preserved.

```tspp:main.tspp
declare const PAGE_PATH: string
  //<- keep-marker
;(()=>{})()
```

```tspp expected
declare const PAGE_PATH: string;
//<- keep-marker
(() => {})();
```

## Prefix Comment Adjacency

### generated prelude comments

Stacked prefix comments are preserved before call expressions.

```tspp:main.tspp
/******/
/* keep-call */ make()
```

```tspp expected
/******/
/* keep-call */ make();
```

### directive comments around use strict

Directive comments around `"use strict"` stay in place.

```tspp:main.tspp
/******/ "use strict" /**/
/******/ a;

function func() {
  /******/ "use strict" //
  /******/ b;
}
```

```tspp expected
/******/ "use strict"; /**/
/******/ a;

function func() {
    /******/ "use strict"; //
    /******/ b;
}
```

## Conditional Arguments

### conditional argument trailing line comment

Trailing line comments after conditional arguments stay on the same argument.

```tspp:main.tspp
cb(
  overflowing ? "absolute top-0" : "relative", // keep-conditional
  parameter
)
```

```tspp expected
cb(
    overflowing ? "absolute top-0" : "relative", // keep-conditional
    parameter,
);
```
