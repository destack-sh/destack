---
title: "Introducing TypeScript++"
subtitle: "Evolving TypeScript into the Last Programming Language"
date: "2026-08-21"
tags: ["language", "runtime"]
author: "Florian"
---

- We need a new _universal_ programming system to write all the world's software: a language that compiles very fast, runs very fast (ideally at machine speed), supports deep static and dynamic analysis, runs seamlssly everywhere (incl. on the web), can run systems software at machine speed, and is legible to humans and agents alike.
- above all, we need a complete system, a method of software production, to reliably produce correct, optimal, integrated software in one standardized way
- fully integrated infrastructure, from the bottom to the top of the "stack". 
- at the centre of it must sit a universal language and runtime.

- If we squint a little, TypeScript is already tantalizingly close to being a serious, native, _universal_ programming language
- If we could get something _like_ TypeScript to run like a native systems language - or at least like a "serious managed" language a la JVM / CLR - that would get us predictable systems-ish performance,  while writing and reading the same language we already like! 
- begone with the need for a separate language and stack to run "backend" or "compute heavy" tasks once we "outgrow" node.js or whatever.
- .. TypeScript also happens to be the very same language that runs the web, the biggest software platform in the world.

- but beyond performance, the opportunities in a standardized, fully integrated computing stack are very interesting
- now that the cost of writing and rewriting code is nearly zero, what can we do
- how can we build better, correct, integrated software systems
- the world is going to run on software, even more so than now, how do we make sure that software is doing what we want?
- (hint: agents are just software, too)
- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale

---

# Why

- the history of programming is one of rising levels of abstraction, from literally wiring up vacuum tubes to punch cards to machine code to assembly to C to Java to TypeScript
- climbing the ladder of abstraction is broadly attractive, as we remove ourselves from the troubling burden of having to actually spell out what _exactly_ it is we want the machine to be doing
- it follows then, that we might want to go all the way, that the ultimate sophistication of programming is not programming at all, but like "talking to a colleague"? 
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by formal languages

- and.. amazingly, dropping code, doing something higher level, this sorta kinda works, sometimes, actually!
- spreadsheets have worked for decades, game dev people have been doing this for a while, we have now figured out a new way to prompt simple software into existence
- (historically, when some more "accessible" programming-ish becomes too common, the "real" programmers no longer consider it programming. thus, excel is not "programming", just like image classification is not AI)

- the trouble is, in software, there is no meaningful separation of the system and its specification, and there is no magic abstraction on top of code that will solve all our problems - if the job is solving novel problems.
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, "APIs will replace everything", etc.)

- think the future of software development is lots more like game development
- sometimes you write engine, sometimes you write scripts
- lot of the time is just iterating in some more interactive editor
- sometimes you "play" the game, sometimes you pop out to edit the game
- lots of internal tools and proprietary

## Why Care About Code

- why should we even think about code at all if it can all be AI written anyway
- feels much like asking why think about materials when a crane will assemble your house anyway
- idea of just generating bytecode _directly and only_ is fanciful and basically a meme
- it makes very little sense, why would we waste tokens on less maintainable code with fewer guarantees
- there is a reason humans have evolved higher level languages, and while we may not settle at the exact same level of abstraction (indeed, TS++ draws the lines a little differently), it will almost certainly not be "generate machine code directly" for general purpose software

- there is not a single test, or suite of tests, mathematical proof, or specific gate that you can run to convince me that some non-trivial program is correct (irrespective of human written or not)
- correctness = alignment + visibility
- correctness is ultimately about alignment, and we can only align on what we can see
- I don't know what I want until I see it, and I also don'T know what I _don't_ want until I see it
- correctness is iterative, the shape is changing
- correctness must be specified acrosss many layers to systematically exclude all the things we do _not_ want

- our tools for writing, interacting with, understanding code are pretty primitive
- if everything is code, how do we make sure it's the right code?
- "oh just have the AI tell you if the code is right" but again what is right?
- (in this sense, the "alignment problem" feels much more like a product and legibility problem, and certainly not _merely_ an intelligence problem, which is short term bearish but long term very bullish)
- this does not magically go away with more abstractions or "smarter AI"
- understand the shape of software and the space of all possible software
- code is going to run everything (literally)
- much better static and dynamic analysis
- study software and code
- uncover the true nature of software

- incremental, granular precision
- who measures the measurer? where is the kernel of truth?
- an equivalent problem with proof systems (hello Gödel)
- alignment requires visibility, we cannot align what we cannot see
- need a common vocabulary
- visibility and precision must be built on a solid foundation, incrementally
- precision requires looking at the code, systems
- .. from many angles, in detail, high low, in motion, statically, all sorts of dynamics, ...
- incremental granularity (a la casey muratori)
- you can't engineer precision and alignment (i.e. understanding) into a system post-hoc (or at least, only with great difficulty that far exceeds the cost of doing it properly from the start)

## Why "Human-First"

- there is a popular current of "agent first design", and a bunch of developer-adjacent tools are being rebuilt to become "agent native".
- I don't really know what that means, beyond good APIs, high performance, and .. just building software in the way we should have done anyway.
- human-first design

- so "agent native" makes for good marketing and pitch decks, but means little in practice
- most things that are "good for agents" - fast iteration, clean boundaries, programmable software - are good for humans too, we just haven't had the opportunity to the big rewrite until now
- and there also just hasn't been a goodopportunity to reconsider deepset habits yet 

- programming is fundamentally about problem solving
- common theme seems to be in _removing_ me from the details, and "just have humans give high level direction"
- I've tried that, it doesn't work, I don't want to do that
- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.
- we used to solve problems by a manual next-character-predictor with a keyword, now we can often work at a higher level.
- but we must still understand, or the software sits in some weird disconnected castle in the sky that serves nobody
- (well over a year of "vibe coding" has shown this pretty concolusively)

- there is something beautiful about doing the most with the fewest possible parts
- a minimal, simple language like C or even Go - though very few people would have called either "minimal" at the time they were introduced - is elegant
- it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases
- the carcinisation of (managed) languages
- universalism, complete vs minimalism, and expressivity
- Go and generics, Java / C# and unsafe / structs / ref, ...
- JVM/CLR by default, Rust on demand

## Why Not Reinvent _Everything_

- there is already wide range of prior art in the realm of "TS ergonomics with systems performance", but that is just one aspect of what we'Re trying to do here
- so, before doing something new, the first question is: why not extend what already exists
- static hermes, assembly script, ...
- all in - not incrementally adoptable.
- (though we do have C ABI ofc)
- fully standardized across the stack

- if we finally have the unique opportunity to build a new programming system, why not just .. throw everything away and start from scratch?
- there are all these suboptimal choices embedded deep into contemporary programming systems
- soo "why not fix all the problems"
- what is the "*ideal* system"
- "second system effect"
- "boiling the ocean"
- if we can port to whatever we want, why not do something entirely new?

- why innovate here? what is the ideal final system?
- where do we begin change? where do we want to end up?
- what is the ideal final system?

- pragmatic perfection
- I don't want to learn your very smart totally new and totally different language
- I like imperative programming
- I want to use the Web, basically TypeScript, and build with stuff I'm familiar with
- I want to use what I already know, with minimal new learning
- Predictable, known behavior - even if imperfect - is better than something totally new, theoretically perfect thing
- (besides, we usually figure out that the grass isn't quite greener anyway..)

- colored functions are fine, and nice and familiar, it's just an effect
- Promises are fine actually
- number is okay as a type actually
- bigint and string are fine
- all in all, it's fine, and most importantly: it's familiar
- (... and it's how the web works!)

- the ergonomic ladder of TS++ between TS -> Rust
- don't try to be cute or clever or fancy
- don't "fix" what's not badly broken
- only use boring ideas already proven by other languages / libraries / ..

---

# How

- we live in an age of miracles, and we can now actually _ask_ the computer to build a typescript compiler  
- .. and it will go off and after a few hours it will (hopefully) come back with something that maybe kinda sorta works! 
- (it's not entirely clear what to _do_ with 1M lines of unaudited C code, but the tests it wrote are green. success?) 
- the more interesting question however is: now that we can translate software with ease, now that most code is no longer written directly by humans, and now that .. which language and stack should we end up with?

- so what do we want, what do I want?
- well some good 15+ years writing software in all kinds of language, I have accumulated a wishlist:
- basically: TS but JVM/CLR with som Rust-y bits? kinda? like that's actually it.
- what is the minimum set of changes / additions we need to good prior art to get what we need
- safe, sound, predictable
- about two dozen or so key decisions to be made when building "typescript++"
- and they roughly split into: how do we support which types, which expressions do we add, how do we deal with memory and layouts, and where does any of this actually run

## Types

- keep muscle memory
- make it strict and sound
- no footguns, remove weirdness
- stable and deterministic, incrementally compilable
- strict boundaries
- make it fast
- cover as much as possible with type algebra (much faster than macros)

### Strictest TS

- TS++ fashions itself as a "superset of a strict subset of TS", which is vaguely reminiscient of the relationship between C and C++
- the obvious first cut is to remove any express soundness holes from TS. 

### Soundness

- no `any`, no `as` (where that is unsound), no array holes, no predicate functions (e.g. `isUser(user: any): user is User` is unsound)
- and certainly no dynamic JS shenanigans or monkey patching, so goodbyte `__proto__` or anything like that
- no `Object.isOwnProperty`, ...
- no "truthiness"; conditionals always take booleans
- need strict, sound TS with predictable module boundaries and type behavior

Many of the most egregious flags don't even come up in TS++ because we don't allow the craziest bits (there is no `any`, there are no computed field accessors, etc.), but the baseline of TS++ is spiritually similar to the following `tsc` flags:
- `strict`: Of course.
- `isolatedDeclarations`: Exported declarations need annotations.
- `noImplicitOverride`: Members overriding a base-class member need to use `override`.
- `strictFunctionTypes`: Checks function parameters covariantly.
- `strictNullChecks`: `null` and `undefined` are distinct types.

### Primitives

- number, yes, but int32, int64, float32, character too
- no real symbol use case left, so no `symbol` or `unique symbol`
- string and bigint are just regular classes (String, BigInt)
- variable sized integers
- isize / usize
- (sequence collections default to isize instead of number)

### Enums

- enums are reasonably simple
- string and integer
- auto enum

### Arrays, Slices and Tuples

- tuples, slices, inline arrays and the rest
- no more array tuples (need to free up `[T]` and `[T; N]`)
- fixed arrays `[T; N]`

### Classes

- classes are generally pretty straightforward, it's just about which tradeoffs do we want?
- JVM? C++? Go?
- zero overhead? vtable pointers? explicit or implicit virtual?
- allocation metadata sidetable on the heap / runtime
- classes are reference types by default, alias freely

### This

- in TS, like in many managed languages, we can just omit "this" in a method and it will just default to the aliasing managed reference
- we support this ofc as well:
- `this` = `&exclusive T` for value types
- `this` = `Managed<T>` for reference types
- implicit and explicit this
- value and borrowed forms

### Nominality

- usually use symbol branding in TS, which is kinda icky
- proper nominality and newtypes
- newtype, newtype traits

### Readonly

- deep readonly
- const is *not* readonly (just like in TS)

### Algebra

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`? (no, has to match exactly, in order)
- Pick, Readonly, ...
- interval types

### Unions

- sum types
- regular unions
- nominal and structural discriminated unions

### Interfaces

- structural interfaces are a key part of typescript
- `type` vs `interface`
- how dynamic do we want to go
- shape mutation
- excess properties
- declaration exprsesions
- dynamic prototypes
- all sorts of JS hacks that everyone hates anyway
- `Record` is read-only

### Funky Signatures

- index
- call
- construct
- can I read through index signatures? can I call through them?
- index signatures
- call signatures

### Generics and Variance

- `T extends string` -> `T: string`
- stay the same basically
- in, out, in out, measured variance
- generalised `const` parameter for value generics (literal types!)

### Narrowing

- instanceof, typeof, is
- type narrowing as usual, narrowing is just doing runtime type checking
- typeof in type position
- is for type queries
- instanceof for classes

## Expressions

- keep all the ergonomics and muscle memory
- remove some legacy weirdness
- expressions as values

### TSX

- tag based trees are pretty useful and broadly applicable
- there are other ways of doing UI, but this is a pretty good one, and it's *very* familiar
- trees, generalised tree litearls,
- lowercase tree builders, ..?
- (unfortunately this also means keeping TS ambiguity around..)

### Patterns and Match

- patterns
- match
- catch match
- `Sequence` type
- No Computed Keys
- no `obj[expr]` where `expr` is dynamic
- ... except for dynamic index signatures where it types as `V | undefined` (via dynamic.find)
- ranges: `..`

### Decorators

- TS already sorta kinda has decorators, sometimes
- extended placement
- decorators on expressions
- newtypes as decorators
- incl. union newtypes
- queryable
- `@if` static gating

### No Exceptions, Only Results

- basically all changes from TS++ to TS are about soundness and strictness, this one is a little more subjective, but given the pain caused.. exceptions most die. we cannot have an invisible side channel infecting everything in the last computing stack
- panic/unwind still exists like in rust but that's worker-scoped, not normal recovery
- most subjective of the bunch
- Try operator, ? ambiguity because TS
- but exceptions have proven troubling over and over and over again
- checked exceptions are even worse
- the only sane error handling method is the Swift-y Rust-y ? operator 

### Try-Catch-Finally

- familiar try / catch / finally syntax still works though!
- catch (e) is all Try error residuals
- catch match (e) as the ergonomic switch

### Extensions

- like `impl` in Rust but a little broader
- inherent, anonymous, named extensions

### Operator Overloading

- serious math-y applications want operator overloading
- `Add`, `Subtract`, `Multiply`, `Divide`, etc.

### Const Evaluation

- originally envisioned something closer to Zig's comptime (or even Jai's version of it)
- `const <expr>` and `const { ... }` for comptime evaluation
- `const function` for comptime functions

### Functions, Lambdas and Captures

- lambdas (fat pointers with env)
- `Function`, `^Function`, `&Function`
- `@capture`

### Async, Promise, Tasks

- proper async
- keep Promise for aliased async
- introduce Task for structured affine concurrency (same async/await model)
- (Promise = managed class, Task = value type, Promise requires aliasable / copyable type)
- fiber-based execution (e.g. JVM's new model)

### Panics, Traps

- overflows / underflows
- out of bounds
- deliberate unreachable

### Context, ContextVars

- like Python
- but for all bindings

### Documentation

- doc comments on expressions too

## Memory

- TS has no real direct way to control memory shapes
- we can trivially restrict to closed shapes, which buys us predictable layouts
- but sometimes we want even more

### Representation

- so, TS++ should behave as much as TS as we can physically manage while keeping sane and predictable performance _and_ behavior
- (and something we can actually build into a good toolchain)
- @repr
- layout

### Local and Shared Memory

- generalise SharedArrayBuffer and friends?
- worker-first, local-first, shared-nothing-first memory model

### Structs and Value Types

- every serious programming language eventually cares about memory layout
- need fixed no overhead shapes

### Ownership

- bare T just means whatever the default form is. preserve TS behavior
- reference types are reference types, value types are value types
- ^T, T, &T, *T, ...
- Managed<T>, Owned<T>, ...
- references and pointers
- arrghh yes seriously pointers in TypeScript let's go

### Borrowing

- if we want value types and we want to pass them around, we need some form of borrowing
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, 

### Lifetimes

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- tried a bunch of things to make this more TS-native, but ultimately, the Rust model really is best (inference only locally within functions, no induced generics beyond that)

### Access, Mutability, Exclusive

- readonly
- &T default to mutable
- &readonly for explicit readonly
- "third rung" on the mutability ladder
- overwrite stability
- (what about data races..? lints / DST / ...)
- we can now distinguish "readonly, non-exclusive", "mutable, non-exclusive", "mutable, exclusive"
- exclusive ownership
- worker-local, borrowing

### Memory Type Algebra

- WithAccess
- PlaceOF
- ...

## Runtime

- again keep conceptual muscle memory
- as with everything else try to keep as familiar as possible
- json based, json is nice, let's use use that
- package.json is okay, let's just use that -> destack.json
- familiar mental model, just combine the disparate pieces

### Burning the Boats

- No Backward Compatibility
- first and most serious cut is to drop support for existing .ts/.tsx alltogether
- no NPM, no JS bridge, no TS "best effort", no fallbacks, nada.
- standardization, integration, .. the whole thing only works with a blank slate
- (I had to figure this out the hard way)

### destack.json

- combine electron, expo, package.json, Cargo.toml, ...

### ESM Modules

- strictly ESM imports and exports
- no CommonJS
- no export / import type though

### Worker-first

- retain local / worker isolation as the primary model
- use Workers for structured concurrency
- (maps to threads N:M)

### Effects / Bindings

- proper colored functions
- stdlib based on explicit @bindings
- effect tracking
- @binding

### Durability

- rewind, fork

### Policy

- control bindings

### import.meta

-

### Conditions

- Generalised Module
- x.ds, x.test.ds, x.whatever.ds

## So

- so what
- first, most central piece of the puzzle
