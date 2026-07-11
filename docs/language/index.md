---
title: Language
description: The Destack language and its relationship to strict modern TypeScript.
order: 20
---

# Language

Our `.ds` ("TypeScript++") is a superset of a "strict modern" subset of `.ts` (TypeScript), similar _in spirit_ to familiar ecosystem extensions like `.svelte` or `.vue`.
Generally, existing TypeScript and TSX _just works_ **if** it is sound _and_ uses no exceptions.
Fortunately, strict TypeScript is already a best practice - it's what you get when enabling the recommended soundness flags in TSC (mostly) - and converting implicit exceptions to explicit results is a trivial (and worthwhile) one-shot transformation.

There are solid arguments that a language should be minimal like Zig or Go or even C, though programmers 50 years ago would not have called them "minimal" by any stretch.
Ultimately, we do not believe "language minimalism" to be pragmatic for the universal language and toolchain we want: Destack aims to be a _complete_ (and coherent and pragmatic) language, not a _minimal_ language.
And since we needed _some_ additions anyway, we took the opportunity to round out the language with modern ergonomics like patterns, operator overloading, reflection, and comptime.
