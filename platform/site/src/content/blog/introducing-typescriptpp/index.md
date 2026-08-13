---
title: "Introducing TypeScript++"
subtitle: "Making TypeScript the Last Programming Language (in Alpha)"
date: "2026-08-18"
tags: ["language", "runtime"]
author: "Florian"
---

# Introduction

- We need a new _universal_ programming language: a language that compiles very fast, is easy analyze statically and dynamically, runs seamlssly on the web, and can run native systems software at machine speed.
- TypeScript is tantalizingly close to being a serious, native, _universal_ programming language
- (this shouldn't be _that_ surprising considering its forebearer was loosely based and slowly evolved next to Java, and ofc course its creator is _also_ the author of C#, itself a very respectable and pretty fast language)
- we would like typescript to run like a native systems language - or at least like a "serious managed" language a la JVM / CLR - very much, since that would get us predictable systems-ish performance,  while writing and reading the same language we already like
- .. and the very same language that runs the web, which happens to be the biggest software platform in the world.

# Why

- we have long tried to rise higher up the ladder of abstraction in code, we have also tried to somehow remove ourselves from the troubling burden of having to actually spell out what exactly it is we want the machine to be doing
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by formal languages
- and, this sorta kinda works, sometimes, actually! 
- spreadsheets are old, game dev people have been doing this for a while, we have now figured out a new way to prompt simple software into existence

- the trouble is, in software, there is no meaningful separation of the system and its specification
- there is no magic abstraction on top of code that will solve all our problems
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, etc.)
- now, many of these higher level specifications have legitimate use cases

- why not extend what already exists
- static hermes, assembly script, ...

## Why Human First Design

- there is a popular current of "agent first design", but I don't really know what that means, beyond just building software in the way we should have done anyway.
- human-first design
- curious trend of "agent native" programming tools
- common theme seems to be in _removing_ me from the details, and "just have humans give high level direction"
- I've tried that, it doesn't work, I don't want to do that
- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.

- tools for those who still care
- on one hand, I'm lazy, and don't want to learn new stuff unless absolutely necessary (or really interesting)
- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale

- there is not a single test, or suite of tests, mathematical proof, or specific gate that you can run to convince me that some non-trivial program is correct (irrespective of human written or not)
- correctness = alignment + visibility
- correctness is ultimately about alignment, and we can only align on what we can see
- I don't know what I want until I see it, and I also don'T know what I _don't_ want until I see it

- correctness is iterative, the shape is changing
- correctness must be specified acrosss many layers to systematically exclude all the things we do _not_ want

- incremental precision
- who measures the measurer? how do we know that the 1kg calibration stone is really exactly 1kg?
- alignment requires precision
- precision must be built on a solid foundation, incrementally
- precision requires looking at the code, systems
- .. from many angles, in detail, high low, in motion, statically, all sorts of dynamics, ...
- incremental granularity (a la casey muratori)
- you can't engineer precision and alignment (i.e. understanding) into a system post-hoc (or at least, only with great difficulty that far exceeds the cost of doing it properly from the start)

## Why Does It Need to Be "Universal"

- universalism, minimalism, and expressivity
- there is something beautiful about doing the most with the fewest possible parts
- a minimal, simple language like C or even Go - it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases
- the carcinisation of (managed) languages
- Go and generics, Java / C# and unsafe / structs / ref, ...
- we want beautiful code, we want minimal code, we want expressive code - these are not in conflict, we need expressivity for beauty, we need clarity for brevity, etc.

```ds
const foo = "Hello, World!";
const foo = &foo;
console.log(foo + *foo)
```

what do we need out of a language stack, ...
requirements, wishlist:
 - familiar to the majority of developers
 - runs directly on the web
 - capable of running natively at machine speed

- why a new language
- why fuse typescript and rust
- why a universal language
- why now

- but alas, hardware is real, and if we want to make fast software, we need to control those low level pesky details somehow
- typescript is tantalizingly close to a systems language
- remove all the dynamic / JS baggage, add a little bit of layout and memory control, and we're looking at a surprisingly presentable low level language

## Why "Combine" TypeScript and Rust

- JVM/CLR by default, Rust on demand
- okay it's basically a meme at this point
- what do we mean by "optimal" and why does it even matter
- we're going to run a _lot_ more software, and ideally, we're also going to run a lot of it in "simulation" and speculative modes - the faster we can do this, the better, and it really adds up
- performance aware programming
- *not* about esoteric data structures or curious
- just want to get within the _ballpark_ of what the amazing machines we have are actually capable of
- currently wildly inefficient because they're pointer chasing across unpredictable, poorly laid out memory

- ownership systems and tight memory control
- lots of interesting ways to make this work
- pragmatic perfection
- all things considered, if you take this premise I have laid out and contrast it with the actual Destack design, it's quite conservative
- I'm not proposing a radical change in how we program, necessarily, or even any wild new programming concepts that don't already exist. the language is quite conservative, and os on
- it's just putting it all togetherin a coherent and sensible way

- the ergonomic ladder of TS++ between TS -> Rust

## Why Stay Within the Lines

- don't try to be cute or clever or fancy
- don't "fix" what's not badly broken
- only use boring ideas already proven by other languages / libraries / ..

- "second system effect"
- "why not fix all the problems"
- "boiling the ocean"
- the "ideal system"
- if we can port to whatever we want, why not do something entirely new?
- safety, safety, and "safety"
- aliasing mutable borrows, the loss of a bit of entropy with `&T`

- colored functions are fine, and nice and familiar, it's just an effect
- Promises are fine actually
- number is okay as a type actually
- bigint and string are fine
- all in all, it's fine, and most importantly: it's familiar
- (... and it's how the web works!)

# How

- we live in an age of miracles, and you can now actually ask your computer to build a typescript compiler *for you*! 
- and it will go off and after a few hours it will (hopefully) come back with something that maybe kinda sorta works! 
- (it's not entirely clear what to _do_ with 1M lines of unaudited C code, but the tests it wrote are green) 
- the more interesting question however is: now that we can translate software with ease, now that most code is no longer written directly by humans, and now that .. which language and stack should we end up with?

- what is the minimum set of changes / additions we need to good prior art to get what we need
- safe, sound, predictable
- about two dozen or so key decisions to be made when building "typescript++"
- how dynamic do we want to support?
- do we want to support "escape hatches"? 
- any sort of backward compatibility

## No Backward Compatibility

- first and most serious cut is to drop support for existing .ts/.tsx
- no NPM, no JS bridge, no TS "best effort"

## destack.json

- combine electron, expo, package.json, Cargo.toml, ...

## Obviously, No Soundness Holes

- any
- sneaky casts
- JS shenanigans
- monkey patching
- array holes
- no predicate functions (e.g. `isUser(user: any): user is User` is unsound)

## Strictest TS

- need strict, sound TS with predictable module boundaries and type behavior
- `strict`
- `alwaysStrict`
- `allowUnreachableCode: false`
- `allowUnusedLabels: false`
- `exactOptionalPropertyTypes`
- `isolatedDeclarations`
- `isolatedModules`
- `noFallthroughCasesInSwitch`
- `noImplicitAny`
- `noImplicitOverride`
- `noImplicitReturns`
- `noImplicitThis`
- `noPropertyAccessFromIndexSignature`
- `noUncheckedIndexedAccess`
- `strictBindCallApply`
- `strictBuiltinIteratorReturn`
- `strictFunctionTypes`
- `strictNullChecks`
- `strictPropertyInitialization`
- no "truthiness"; conditionals always take booleans

## ESM Modules

- strictly ESM imports and exports
- no CommonJS
- no export / import type though

## Proper Primitives

- number, yes, but int32, int64, float32, character too
- tuples slices inline arrays and the rest

## Nominality

- usually use symbol branding in TS, which is kinda icky
- proper nominality and newtypes
- newtype traits

## TSX

- tag based trees are pretty useful and broadly applicable
- there are other ways of doing UI, but this is a pretty good one, and it's *very* familiar
- trees, generalised tree litearls,
- lowercase tree builders, ..?

## Patterns and Match

- expressions as values
- patterns
- match
- catch match

## Decorators

- extended placement
- newtypes as decorators
- incl. union newtypes
- queryable
- @if static gating

## No Exceptions, Results Only

- most subjective of the bunch
- but exceptions have proven troubling over and over and over again
- checked exceptions are even worse
- the only sane error handling method is the Swift-y Rust-y ? operator 

## Structural Interfaces, `Dynamic` (?)

- shape mutation
- excess properties
- declaration exprsesions
- dynamic prototypes
- all sorts of JS hacks that everyone hates anyway
- `Record` is read-only

## Extensions

- inherent, anonymous, named extensions

## Operator Overloading

- serious math-y applications want operator overloading
- `Add`, `Subtract`, `Multiply`, `Divide`, etc.

## Classes, Yes, but Which Ones

- JVM? C++? Go?
- zero overhead? vtable pointers?

## Objects and Type Algebra

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`?

## `Dynamic` and Structural Interfaces

- how dynamic do we want to go
- index signatures
- can I read through index signatures? can I call through them?
- call signatures

## Generics and Variance

- stay the same basically
- in, out, in out, measured variance
- new `comptime` parameter for value generics

## Structs and Value Types

- every serious programming language eventually cares about memory layout
- need fixed no overhead shapes

## Ownership

- bare T just means whatever the default form is
- reference types are reference types, value types are value types
- ^T, T, &T, *T, ...
- Managed<T>, Owned<T>, ...

## Borrowing

- if we want value types and we want to pass them around, we need some form of borrowing
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, 

## Lifetimes

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- tried a bunch of things to make this more TS-native, but ultimately, the Rust model really is best (inference only locally within functions, no induced generics beyond that)

## Access, Mutability, Exclusive

- readonly
- &T default to mutable
- &readonly for explicit readonly
- "third rung" on the mutability ladder
- exclusive ownership
- worker-local, borrowing

## Local and Shared Memory Spaces

- SharedArrayBuffer and friends?
- worker-first, local-first, shared-nothing-first memory model

## Async, Promise, Tasks

- proper async
- keep Promise for aliased async
- introduce Task for structured affine concurrency (same async/await model)
- fiber-based execution (e.g. JVM's new model)

## Panics, Traps

- overflows / underflows
- out of bounds
- deliberate unreachable

## Automatic, Implicit Effects

- proper colored functions
- stdlib based on explicit @bindings
- effect tracking
- @binding

## Durability

- rewind, fork

## Generalised Module

- x.ds, x.test.ds, x.whatever.ds

## Macros

## Reflection
