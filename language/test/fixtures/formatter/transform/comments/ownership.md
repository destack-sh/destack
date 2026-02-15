# Comment Ownership

Tests for comment retention and attachment across formatter boundary decisions.

## Optional Chain Boundaries

### block comment before optional call

Block comments before optional calls stay attached to the callee boundary.

```ts:main.ts
const value = call /* keep-call */ ?.()
```

```ts expected
const value = call /* keep-call */?.();
```

### line comment before optional call

Line comments before optional calls stay attached to the full expression.

```ts:main.ts
const value = call // keep-line
?.()
```

```ts expected
const value = call?.(); // keep-line
```

## Statement Boundaries

### if trailing condition comment

Line comments after conditions stay attached to the condition line.

```ts:main.ts
if (ready) // keep-condition
    run()
```

```ts expected
if (ready)
    // keep-condition
    run();
```

### class field trailing block comment

Trailing block comments on class fields stay with the field declaration.

```ts:main.ts
class Box {
    value = 1 /* keep-field */
    next = 2
}
```

```ts expected
class Box {
    value = 1; /* keep-field */
    next = 2;
}
```

## Type Boundaries

### union arm line comment

Comments between union arms are preserved between the same arms.

```ts:main.ts line-width=20
type Value = string | // keep-union
    number
```

```ts expected
type Value =
    | string // keep-union
    | number;
```

### union arm block comment

Block comments between union arms are preserved in place.

```ts:main.ts line-width=20
type Value = string /* keep-union */ | number
```

```ts expected
type Value =
    | string /* keep-union */
    | number;
```

### union arm doc block comment

Doc block comments between union arms are preserved as arm leaders.

```ts:main.ts line-width=80
export type Value = /** keep-doc
 */
| { ok: true }
| { ok: false; value: bigint | null };
```

```ts expected
export type Value = /** keep-doc
     */
    { ok: true } | { ok: false; value: bigint | null };
```

## Variable Declarations

### variable trailing marker line comment

Marker comments at declaration tails are preserved.

```ts:main.ts
declare const PAGE_PATH: string
  //<- keep-marker
;(()=>{})()
```

```ts expected
declare const PAGE_PATH: string;
    //<- keep-marker
(() => {})();
```

## TSX Ternary Branches

### tsx ternary branch comments

Inline comments inside TSX ternary branches are preserved on both sides.

```tsx:main.tsx line-width=40
const node = <div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```tsx expected
const node = (
    <div>
        {
            isVideo ? (
                <Video />
            ) : (
                /* keep-video */ <Image />
            ) /* keep-image */
        }
    </div>
);
```

### tsx ternary alternate block comment

Block comments inside alternate TSX branches are preserved.

```tsx:main.tsx
const Component = () => (
  <div>
    {"error" ? (
      <Error />
    ) : (
      <Success />
      /* keep-inside-branch */
    )}
  </div>
)
```

```tsx expected
const Component = () => (
    <div>
        {"error" ? (
            <Error />
        ) : (
            <Success />
            /* keep-inside-branch */
        )}
    </div>
);
```

## Directive Adjacency

### generated directive prelude comments

Directive-like prefix comments are preserved before pure annotations.

```ts:main.ts
/******/
/*#__PURE__*/ make()
```

```ts expected
/******/
/*#__PURE__*/ make();
```

### directive comments around use strict

Directive comments around `"use strict"` stay in place.

```ts:main.ts
/******/ "use strict" /**/
/******/ a;

function func() {
  /******/ "use strict" //
  /******/ b;
}
```

```ts expected
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

```ts:main.ts
cb(
  overflowing ? "absolute top-0" : "relative", // keep-conditional
  parameter
)
```

```ts expected
cb(
    overflowing ? "absolute top-0" : "relative", // keep-conditional
    parameter,
);
```
