---
title: "Introducing TypeScript++"
subtitle: "Evolving TypeScript into the Last Programming Language"
date: "2026-08-21"
author: "Florian"
---

- how do we build correct, optimal, integrated software?
- We need a new _universal_ programming system to write all the world's software: a language that compiles very fast, runs very fast (ideally at machine speed), supports deep static and dynamic analysis, runs seamlssly everywhere (incl. on the web), can run systems software at machine speed, and is legible to humans and agents alike.
- above all, we need a complete system, a unified method of software production, to reliably produce correct, optimal, integrated software in one standardized way
- fully integrated infrastructure, from the bottom to the top of the "stack". 
- at the centre of it must sit a universal language and runtime.

- If we squint a little, TypeScript is already tantalizingly close to being a serious, native, _universal_ programming language
- If we could get something _like_ TypeScript to run like a native systems language - or at least like a "serious managed" language a la JVM / CLR - that would get us predictable systems-ish performance,  while writing and reading the same language we already like! 
- begone with the need for a separate language and stack to run "backend" or "compute heavy" tasks once we "outgrow" node.js or whatever.
- .. TypeScript also happens to be the very same language that runs the web, the biggest software platform in the world!
- but of course, it would have to really *feel* like TypeScript, not just "look" like TypeScript! as much as possible, TypeScript semantics - far beyond the surface syntax - should be preserved for this to really be a day one language.

- but beyond performance, the opportunities in a standardized, fully integrated computing stack are very interesting
- now that the cost of writing and rewriting code is nearly zero, what can we do
- how can we build better, correct, integrated software systems
- homoiconic software
- "unreal engine for software production"?
- what are the decisions we can fix and freeze, sort of like how the advent of opinionated formatters put an end to a whole series of unproductive discussions?
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

- the trouble is, in software, there is no meaningful separation of the system and its specification, - and thus there is no magic abstraction on top of code that will solve all our problems - if the job is solving novel problems.
- (the granularity of the specification to care about depends strongly on how standardized the solution is. fine tuning a character controller? better control every bit of entropy. building an email sender? just plop in a framework, we already do this.)
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, "APIs will replace everything", etc.)

- think the future of software development is lots more like game development
- sometimes you write engine, sometimes you write scripts
- lot of the time is just iterating in some more interactive editor
- sometimes you "play" the game, sometimes you pop out to edit the game
- lots of internal tools and proprietary

## Why Care About Code

- why should we even think about code at all if it can all be AI written anyway
- feels much like asking why think about materials when a crane will assemble your house anyway
- or why think in numbers when calculators exist
- idea of just generating bytecode _directly and only_ is fanciful and basically a meme
- it makes very little sense, why would we waste tokens on less maintainable code with fewer guarantees
- there is a reason humans have evolved higher level languages, and while we may not settle at the exact same level of abstraction (indeed, TS++ draws the lines a little differently), it will almost certainly not be "generate machine code directly" for general purpose software

- fundamentally, there is not a single test, suite of tests, mathematical proof, or specific gate that you can run to convince me that some non-trivial program is correct 
- doesn't matter whether it's human written or not, software is just very complex
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
- make a map

- code is going to run _everything_, even more so than it already does (literally)
- much better static and dynamic analysis
- study software and code
- "software in motion"
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

## Why Design "Human-First"

- there is a popular current of "agent first design", and a bunch of developer-adjacent tools are being rebuilt to become "agent native".
- I don't really know what that means, beyond good APIs, high performance, and .. just building software in the way we should have done anyway.
<!--- "agent native" makes for good marketing and pitch decks, but means little in practice-->
- most things that are "good for agents" - fast iteration, clean boundaries, programmable software - are good for humans too, we just haven't had the opportunity to the big rewrite until now
<!--- and there also just hasn't been a good opportunity to reconsider deepset habits yet -->

- programming is fundamentally about problem solving
- however, the contemporary theme seems to be in _removing_ me from the details, and "just have humans give high level direction"
- I've tried that, it doesn't work, I don't want to do that
- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.
- I want to know, what is the space of all possible software programs to solve my program, and how do I most efficiently get there, where do I go when I'm there, how do I stay in the right place, and so on
- we used to solve problems by a manual next-character-predictor with a keyword, now we can often work at a higher level.
- but we must still understand, or the software sits in some weird disconnected castle in the sky that serves nobody
- (well over a year of "vibe coding" has shown this pretty concolusively)

- there is something beautiful about doing the most with the fewest possible parts
- a minimal, simple language like C or even Go - though very few people would have called either "minimal" at the time they were introduced - is elegant in a way.
- it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases
- the carcinisation of (managed) languages
- however, over time, most serious languages with actual production use evolve an set of common features for building serious software
- Go and generics, Java / C# and unsafe / structs / ref, ...
- JVM/CLR by default, Rust on demand

## Why Not Reinvent _Everything_

- there is already wide range of prior art in the realm of "TS ergonomics with systems performance", but that is just one aspect of what we'Re trying to do here
- so, before doing something new, the first question is: why not extend what already exists
- static hermes, assembly script, ...
- all in - not incrementally adoptable.
- (though we do have C ABI ofc)
- fully standardized across the stack

- if we finally have the unique opportunity to build a completely new programming system, why not just .. throw everything away and start from scratch?
- there are all these suboptimal choices embedded deep into contemporary programming systems
- soo "why not fix all the problems"
- what is the "*ideal* system"
- it is very tempting to get a blank piece of paper and dream up the perfect system
- if "boiling the ocean" suddenly becomes (theoretically) feasible, what sort of landscaping could we do on the software ecosystem
- "second system effect"

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

- so there is the overwhelming sense of the evolution of programming has yielded a current set of pretty good languages, and wildly different ideas - while not necessarily wrong - are at least inherently suspicious
- so you know, colored functions are fine, and nice and familiar, it's just an effect
- Promises are fine actually. microtasks a little weird but whatever
- number is okay as a type actually, it's convenient
- bigint and string are fine, not ideal, but fine

- all in all, it's fine, and most importantly: it's familiar
- (... and it's how the web works!)
- no intention of "fixing" anything that is sound but clumsy and can be trivially linted for 
- more importantly, want a complete language that can represent all the things we need, and then constrain by package / library (but it all has to go togetheraaa)
- so let's just get on with it

---

# How

- so what do we want, what do I want?
- after some good 15+ years writing software in all kinds of languages and domains, what is my wishlist for the ideal language
- basically: TS but JVM/CLR with som Rust-y bits? kinda? like that's actually it.
- what is the minimum set of changes / additions we need to good prior art to get what we need
- safe, sound, predictable
- about two dozen or so key decisions to be made when building "typescript++"
- and they roughly split into: how do we support which types, which expressions do we add, how do we deal with memory and layouts, and where does any of this actually run

- what is the ergonomic ladder of TS++ between TS -> Rust
- don't try to be cute or clever or fancy
- don't "fix" what's not badly broken
- only use boring ideas already proven by other languages / libraries / ..

- TS++ fashions itself as a "superset of a strict subset of TS", which is vaguely reminiscient of the relationship between C and C++
- the obvious first cut is to remove any express soundness holes from TS
- then we figure out how to map the type system to something we can actually compile for real
- and what we need to add to the types and expressions and runtime to make this a serious systems language

## Types

- types are great, type systems are fantastic
- keep muscle memory
- make it strict and sound
- no footguns, remove weirdness
- stable and deterministic, incrementally compilable
- strict boundaries
- make it fast
- cover as much as possible with type algebra
- (TS algebra and generics are much faster than macros since it's essentially a very constrained macro system)

### Soundness

- obviously, the trivial part out the way first: no soundness holes, no dynamic shenanigans, no JS legacy compat
- need strict, sound TS with predictable module boundaries and type behavior
- ideally also make it fast to compile, which requires cleaner boundaries than standard TS gives

- no declaration merging
- no separate type and value spaces (as such)
- no `any`, no `as` (where that is unsound / unchecked)
- no predicate functions (e.g. `isUser(user: any): user is User` is unsound)
- no array holes
- unknown still works as a fat existential

- no dynamic JS shenanigans or monkey patching, so goodbye to `__proto__` or anything like that
- no `Object.isOwnProperty`, `Object.assign`, ...
- no "truthiness"; conditionals always take booleans
- no sequence expressions, who needs sequence expressions

- the more interesting question is how much of TS can we make sound, predictable, and fast.

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
- no more array tuples (need to free up `[T]` and `[T; N]`, arbitrary `[X, Y, ...]` is an error)
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

- no method binding (i.e. no obj.method, instead use () => object.method()) for clarity)

### Nominality

- usually use symbol branding in TS, which is kinda icky
- proper nominality and newtypes
- newtype, newtype traits

### Readonly

- deep readonly
- const is *not* readonly (just like in TS)

### Visibility

- no need for #privateField
- we have private, we just use that
- and it codegens to #privateField on JS targets
- no additional visibiliyt controls

### Algebra

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`? (no, has to match exactly, in order)
- interval types over finite sets (integers)

- utility types!
- mapped types
- Pick, Readonly, ...
- ThisParameterType
- .. all the other utility types

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

- index signatures
    - can I read through index signatures? can I call through them?
    - (index signatures use `dynamic.find` at runtime, which is a linear scan over string equality!)
- call signatures
- construct signatures

### Generics

- trivial syntactic cleanup: `T extends string` -> `T: string`
- stay the same basically
- in, out, in out, measured variance
- generalised `const` parameter for value generics (literal types!)
- monomorph or not to monomorph
- how far? do we monomorph refs?
- JVM / CLR -> Go -> Rust / C++
- build vs release mode

### Narrowing

- type narrowing as usual, narrowing is just doing runtime type checking
- instanceof, typeof, is
- typeof in type position
- is for type queries
- instanceof for classes
- match narrowing

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
- switch still works but match encouraged

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

### Using

- using / async using
- Dispose / AsyncDIspose
- vs Drop

### Extensions

- like `impl` in Rust but a little broader
- inherent, anonymous, named extensions
- E / T, T may be local or imported
- `extension of T`
- `extension E of T`
- `export extension of T`
- `export extension E of T`
- `extension<T> of T`: blanket extension

### Operator Overloading

- serious math-y applications want operator overloading
- `Add`, `Subtract`, `Multiply`, `Divide`, etc.
- `Vector2<float32> + Vector2<float32>`

### Const Evaluation

- originally envisioned something closer to Zig's comptime (or even Jai's version of it)
- originally had a comptime keyword here but was kinda confusing
- `const <expr>` and `const { ... }` for comptime evaluation
- `const function` for comptime functions
- cardinality is measured by usage (sort of like how variance and )

### Functions, Lambdas and Captures

- lambdas (fat pointers with env)
- `Function`, `^Function`, `&Function`
- `FunctionPointer` for raw function pointers without environment
- `@capture`

### Async

- proper async
- keep familiar Promise for aliased async
- introduce Task for structured affine concurrency (same async/await model)
- (Promise = managed class, Task = value type, Promise requires aliasable / copyable type)
- *fiber*-based execution (e.g. JVM's new model)

### Context, ContextVars

- like Python
- but for all bindings
- `Context`

### Panics, Traps

- overflows / underflows
- out of bounds
- deliberate unreachable

### Style and Documentation

- one of the perks of owning the whole toolchain is we can make the parser (and formatter and linter and such) do whatever we want
- doc comments on expressions too, why not
- colored regions?
- logic blocks
- neurotic code styles

## Memory

- TS, following JS, has no real direct way to control memory shapes or allocations
- (though this doesn't stop serious TS programmers to think about hidden class caches and all the brilliantly engineered details of the popular JS engines to keep their software reasonably fast)
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
- local and shared modifier on types
- local and shared modifier on bindings
- local and shared modifier on declarations

- worker-local stuff is .. local (Promise, Task, etc.)
- no need for Send and Sync, basically the 90 degree rotated version of that classic pair

### Structs

- every serious programming language eventually cares about memory layout and allocations
- need fixed no overhead shapes
- no inheritance
- no embedding (unlike Go, Jai)

### Ownership

- Value Types
- with move semantics
- bare T just means whatever the default form is. preserve TS behavior
- reference types are reference types, value types are value types
- ^T, T, &T, *T, ...
- Managed<T>, Owned<T>, ...
- references and pointers
- arrghh yes seriously pointers in TypeScript let's go

### Borrowing

- if we want value types and we want to pass them around, we need some form of borrowing
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, ...
- all types can contain references, just like in Rust

### Lifetimes

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- tried a bunch of things to make this more TS-native, but ultimately, the Rust model really is best (inference only locally within functions, no induced generics beyond that)

### Mutability

- let / const preserve TS meaning
- can take &exclusive only on managed types for const

- readonly
- &T default to mutable
- &readonly for explicit readonly
- "third rung" on the mutability ladder
- overwrite stability
- (what about data races..? lints / DST / ...)
- we can now distinguish "readonly, non-exclusive", "mutable, non-exclusive", "mutable, exclusive"
- exclusive ownership
- worker-local, borrowing

- WithAccess
- PlaceOf
- ...

## Runtime

- again keep conceptual muscle memory
- as with everything else try to keep as familiar as possible
- json based, json is nice, let's use use that
- package.json is okay, let's just use that -> destack.json
- familiar mental model, just combine the disparate pieces

- Burning the Boats
- No Backward Compatibility
- first and most serious cut is to drop support for existing .ts/.tsx alltogether
- no NPM, no JS bridge, no TS "best effort", no fallbacks, nada.
- standardization, integration, .. the whole thing only works with a blank slate
- (I had to figure this out the hard way)

### "Write Once, Run Everywhere"

- yada yada heard it a million times
- (though it did arguably sorta work for Java, and now the web , and maybe WASM, .. mostly)
- want portable
- always build from source?

### destack.json

- oh what is the theoretically ideally package format? toml? txt? magic setup.py? just kidding
- combine electron, expo, package.json, Cargo.toml, ...

### ESM Modules

- strictly ESM imports and exports
- imports, exports, re-exports, defaults, etc. it's all the same
- import data files 
- no async imports / exports
- no CommonJS
- no export type / import type

### Worker-first

- retain local / worker isolation as the primary model
- use Workers for structured concurrency
- (maps to threads N:M)

### Effects / Bindings

- proper colored functions
- stdlib based on explicit @bindings
- effect tracking
- @binding

### import.meta

-

### Conditions

- Generalised Module
- x.ds, x.test.ds, x.whatever.ds

### Documentation

- builtin ish?
- cargo doc?
- jsdoc?

### Topology

- Entity, Edge, ...
- C4, ...

## So

- so what
- first, most central piece of the puzzle
