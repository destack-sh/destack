# parser

Lexer and parser for Destack, TypeScript, and JavaScript.
Transforms source text into the [Destack AST](../ast/README.md).

## Overview

The parser handles `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` files uniformly.
Destack syntax is a superset of TypeScript, so valid TS/JS parses to a valid AST subset.
(Again, see the conformance tests for more details.)

### Two-Phase Parsing

Unlike most TS parsers, we separate the lexing and parsing phases to preserve CST-style information for annotations like decorators and comments better:
1. **Lexer** (`lex/`): Converts source text to tokens
2. **Parser** (`parse/`): Builds AST from token stream via recursive descent

There are tradeoffs: the lexer must be somewhat context-sensitive to handle template strings and tree literals.
But this lets the core parser focus on "main constructs" while annotations like docs and decorators just work automatically.

### "Recursive Descent"

The parser uses recursive descent with operator precedence climbing.
(Why this deserves a special name is unclear: it's just how you write a parser.)

Like most parsers, we use a lookahead and some context to make parsing decisions.
Many syntactic ambiguities in TypeScript (and some more in Destack) require context tracking:

### Error Recovery

The parser tries to continues after errors to report multiple diagnostics per file.
We use synchronization points (`;`, `}`, keywords) to resync after malformed input.

### Tree Literals (TSX)

We support full JSX/TSX compatibility for "tree literals".
This is very annoying, and we borrow some web-land tricks here (with a scanner-ish approach), and some unavoidable backtracking.

```tsx
<Component prop={value}>
    <Child />
    {expression}
</Component>
```

## Annotations

Destack annotations are more flexible than JS/TS or indeed in most programming languages: we support annotating imports, expressions, parameters, members, declarations, etc.
We use a combination of "trivia" tracking for comments / documentation and explicit decorator parsing in legal productions.

Comments, docs, and blanks are attached only in `attach_trivia()`.
`Parser::parse()` always runs `attach_trivia()`.
(For that reason, if a test or tool uses direct entrypoints like `eat_expression` or `eat_block`, it must call `attach_trivia()` before asserting annotation ownership.)

### Rules

We tried hard to come up with a minimal, logical set of rules that cover the entire IR space while enabling prettier-style formatting (as a consumer).
It's not perfect, and JS/TS/TSX/DS are a bit messy in places.
These rules apply uniformly to `comment`, `doc`, and `blank` trivia after lexical classification.

Some terminology:
- `separator-adjacent` means trivia touches a separator token boundary with no intervening attachable owner token.
- `closer-adjacent` means trivia touches a closing delimiter boundary with no intervening attachable owner token.
- `boundary` means a seam where attachment must use boundary positions rather than interior positions.
- `wrapper owner` means the syntactic wrapper node that contains a value node for list and key-value contexts.
- `independent right owner` means a right-hand node that can own trivia directly without relying on an enclosing wrapper fallback.

#### successful-parse scope is normative
These rules are strict for successful parses.
Error-recovery paths are best-effort but must still satisfy single-ownership and no-dup invariants.

#### precedence order is strict
Ownership resolution runs in this order: separator-adjacent rules, closer-adjacent rules, two-sided seam rules, one-sided seam rules, then container or stub fallback.
Separator-adjacent is defined for `,`, `|`, `&`, `as`, `satisfies`, `?`, and `:`.

```ds
call(first /* before-separator: line-postfix on first */, second)

const value = source as /* after-separator: line-prefix on Target */ Target

call(first, second /* before-close: line-postfix-boundary on second */)
```

#### after-separator trivia binds to the next item
If trivia is immediately after a separator token, it attaches to the next item as prefix.

```ds
call(first, /* next-arg: line-prefix on second */ second)

[first, /* next-element: line-prefix on second */ second]

type Value = First | /* next-type: line-prefix on Second */ Second

cond ? /* then-branch: line-prefix on onTrue */ onTrue : onFalse
```

#### before-separator trivia binds to the previous item
If trivia is immediately before a separator token, it attaches to the previous item as postfix.

```ds
call(first /* prev-arg: line-postfix on first */, second)

[first /* prev-element: line-postfix on first */, second]

type Value = First /* prev-type: line-postfix on First */ | Second

const values = [1, 2] /* prev-operand: line-postfix on array */ as const
```

#### before-closer trivia binds to the previous item
If trivia is immediately before a closing delimiter in a non-empty container, it attaches to the previous item as postfix-boundary.
Closer-adjacent applies to `)`, `]`, and `}`.

```ds
call(first /* call-tail: line-postfix-boundary on first */
)

[first /* array-tail: line-postfix-boundary on first */
]

const config = {
    first: 1 /* object-tail: line-postfix-boundary on first property */
}
```

#### same-line two-sided seams bind to the left owner
If trivia has both left and right attachable owners on the same line seam, it attaches to the left owner as postfix.

```ds
a /* same-line: line-postfix on a */ + b

source /* hop: line-postfix on source */ .next()

target /* before-index: line-postfix on target */ [index]
```

#### own-line two-sided seams bind to the right owner
If trivia has both left and right attachable owners on its own line seam, it attaches to the right owner as prefix.

```ds
a +
/* own-line: block-prefix on b */
b

source
// chain-head: block-prefix on .next hop
.next()

cond ?
// ternary-head: block-prefix on onTrue
onTrue : onFalse
```

#### one-sided forward seams bind to the forward owner
If trivia has only a right attachable owner, it attaches to that owner as prefix.

```ds
// file-head: block-prefix on value
value()

call(
    // arg-head: block-prefix on argument wrapper
    value
)

{
    // block-head: block-prefix on first statement
    value()
}
```

#### one-sided backward seams bind to the backward owner
If trivia has only a left attachable owner, it attaches to that owner as postfix.

```ds
value() // tail: line-postfix-boundary on value

const done = true /* eof-tail: line-postfix on declaration */

value()
// eof-own-line-tail: block-postfix on value
```

#### call-and-new paren seams bind to the callee owner
Trivia in the callee-to-`(` seam for `Call` and `New` attaches to the callee owner, including empty argument lists.

```ds
target(/* call-boundary: block-postfix on callee target */)

new Factory(/* new-boundary: block-postfix on callee Factory */)

items.map /* keep: line-postfix on callee items.map */ ((item) => item)
```

#### wrapper owners outrank inner value owners
If a wrapper node and its inner value are both valid candidates for the same seam, ownership goes to the wrapper owner.
Wrapper owners include argument wrappers, property wrappers, parameter wrappers, and import or export specifier wrappers.

```ds
call(/* arg-head: line-prefix on argument wrapper */ value)

({ /* computed-key: line-prefix on property wrapper */ [name]: value })

import {
    first,
    // specifier-head: block-prefix on second specifier wrapper
    second,
} from "mod"
```

#### annotation ordering is source-stable across decorators and trivia
When decorators and trivia attach to the same owner, final annotation order follows source order with no reordering.

```ds
// before: block-prefix comment on declaration
@entity
// after-decorator: block-prefix comment on declaration
class Value {}
```

#### no-side seams bind to container infix, then stub infix
If trivia has no side owner, it attaches to the nearest enclosing container as infix, or to the file stub if no container exists.

```ds
function main() {
    // block-infix: block-infix on block
}

// file-only comment: block-infix on file stub

<div>{/* jsx-container: block-infix on expression stub */}</div>
```

#### blank runs preserve exact line counts
Blank trivia attachments preserve exact blank-run counts and do not collapse line-count semantics.

```ds
let a = 1


let b = 2
```

#### payload normalization is ownership-independent
Ownership never rewrites comment meaning.
Docs may normalize marker syntax while preserving semantic text lines.
Line and block comments preserve payload text and required internal newlines.

```ds
/**
 * line a
 * line b
 */
value()
```

#### ownership invariants are mandatory
Every trivia token must attach exactly once, attachments must be deterministic, owner-local annotation order must follow source order, and overlapping optional-chain candidates must collapse to one final owner with no duplicates.

```ds
// first: block-prefix #1 on value
// second: block-prefix #2 on value

value()

call // single-owner line-postfix-boundary on call callee
?.()

source.first /* opt-boundary: line-postfix-boundary on source.first */
?.second()
```

#### owner-to-position mapping is fixed
Once owner side is chosen, `AnnotationPosition` is derived only from this mapping and never from formatter heuristics.
`right + same-line -> LinePrefix`.
`right + own-line -> BlockPrefix`.
`left + same-line interior -> LinePostfix`.
`left + same-line boundary -> LinePostfixBoundary`.
`left + own-line -> BlockPostfix`.
`container or stub -> BlockInfix`.

```ds
call(/* same-line-right: line-prefix on argument */ value)

left /* interior: line-postfix on left */ + right

left // boundary: line-postfix-boundary on left
+ right

left +
// own-line-right: block-prefix on right
right

value()
// own-line-left: block-postfix on value

{
    // infix: block-infix on block
}
```

### Exceptions

This is a fixed and minimal exception set.
No other ownership exceptions should be added outside this list.

#### as-const token-gap trivia binds to the as-const owner as infix
For TypeScript `as const`, trivia between `as` and `const` attaches to the `AsConst` owner as infix.

```ds
1 as // before-const: block-infix on as-const node
const

1 as
/* between: block-infix on as-const node */
const
```

#### directive comments stick to the following statement
Directive comments like `@ts-ignore` and `@ts-expect-error` attach as prefix to the following statement.

```ds
// @ts-ignore: block-prefix on following statement
value = compute()

// @ts-expect-error: block-prefix on following statement
call(a, b)
```
