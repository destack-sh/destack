---
title: Language
warning: TypeScript++ is experimental and not generally available.
description: Safe, predictable, fast, and - above all - familiar.
---

# Language

<!--# TypeScript++

The univeral final stack will, eventually, require the universal final language.
Surprisingly, we do not quite have that universal complete language yet.
Of course, there are languages you could _contort_ to target all platforms and write everything from systems software to API services to web apps.
But there is a reason nobody runs Rust rust for web apps, even though WASM has existed for a decade.

The closest thing we have to a universal language is TypeScript.
TypeScript is actually pretty great.
Everyone knows TypeScript, and - critically - the web runs on TypeScript (JavaScript).
- the dichotomy between "scripting languages" and "systems languages" no longer makes much sense if it's not humans doing the typing (assuming "compile times" are fast)

TypeScript is already tantalizingly close to being a serious, native, _universal_ programming language.
Naturally, there is serious prior art in the realm of "TS ergonomics with systems performance": Static Hermes, Assembly Script, ...
The shape of TypeScript is conveniently amendable to the (minor) modifications we need to make it analysable and simulatable, with very familiar APIs.

We want, effectively, "TypeScript++".
Kill all the soundness warts, add just enough features added to enable memory safe systems programming, and build out familiar enough serious runtime.
Importantly, the question is _not_ what is the "best theortical version if we did TypeScript all over again", but: "what is the minimum edit distance from TypeScript to a universal language that keeps TypeScript's ergonomics and familiarity, is strict and sound and analysable, and also runs reliably at machine speed?-->

<!-- TODO #Incomplete-->
<!--- TS++ fashions itself as a "superset of a strict subset of TS", which - if you squint - is somewhat reminiscient of the relationship between C and C++.
<!--- there are a lot of interesting details in making "TS++" actually work.
- like how _exactly_ do we combine as much of TS surface feel as possible, while also compiling to a strict sound languaeg? while _also_ enabling up to Rust-level control (and ideally performance)?
- you can read all about it at [docs](/docs/language/)-->

TypeScript++ is a safe, predictable, fast, and _familiar_ scripting _and_ systems language that combines the best of TypeScript with the safety of Rust, and attempts to bridge the gap between the two as gracefully as possible.
To be as easy to try and adopt as possible, TS++ follows existing standards and conventions as much as possible, from the Node/Web-shaped standard library, to package formats (`destack.json` extends `package.json`), to formatter conventions (`.ds` looks almost exactly like `.ts/.tsx` with prettier), to linter rules (many are borrowed from `clippy`, `rustc`, `eslint`, etc.).

As TypeScript++ is designed to supersede TypeScript, it does not attempt to be a drop-in replacement for TS, nor are we interested in any backward compatibility.
The idea is to enable "one-shot migration", meaning that a reasonably competent AI agent as of >=2026 should be able to look at a single file and transcribe from `.ts` into `.ds` without difficulty, requiring at most small global patchup.
