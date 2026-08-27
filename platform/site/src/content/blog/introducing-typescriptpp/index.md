---
title: "Introducing TypeScript++"
subtitle: "Evolving TypeScript into the Last Programming Language"
date: "2026-09-14"
author: "Florian"
---

- there is a lot of software, and there is about to be a whole lot more
- most of it is not very good
- slow, buggy, clunky,
- dependency sprawl
- supply chain attacks
- very hard to run software locally
- not really hackable
- unfortunately, just regurgutitating the software we already have isn't going to be _great_
- so, how do we build correct, optimal, integrated software?
<!--- with exciting new capabilities that are even harder to get right-->

- produce much more software than any one could ever meaningfully review
- what does higher order programming look like? what does it even mean?
- what should higher order progrmaming _feel_ like?
- if code is so cheap, why can't we make it really good?
- if software is so cheap, why can't we make _that_ really good?

- will soon hasve the capbility to rewrite software from and to any language
- so what is the final destination?
- where do we want to end up?
- what characteristics would the final language have?
- would it look like an existing one, or something totally different?

<!--- We need a new _universal_ programming system to write all the world's software-->
- well, ideally:
- a language that compiles fast, runs fast, supports analysis, runs everywhere
- above all, a language that is boring, "works as you would expect", so we can innovate in other places
- I don't want to learn new stuff in this area, I want to make more of the stuff I already know
- ideally we want something that can run at the web properly *and* can run systems software at machine speed, and is legible to humans and agents alike.
- above all, we need a complete system, a unified method of software production, to reliably produce correct, optimal, integrated software in one standardized way
- fully integrated infrastructure, from the bottom to the top of the "stack". 
- at the centre of it must sit a universal language and runtime.

---


- beyond performance, the opportunities in a standardized, fully integrated computing stack are very interesting
- now that the cost of writing and rewriting code is nearly zero, what can we do
<!--- how can we build better, correct, integrated software systems-->
- homoiconic software
- put the "engineering" into "software engineering"
- "unreal engine for software production"?
- what are the decisions we can where we can settle on one set of good, coherent options, sort of like how the advent of opinionated formatters put an end to a whole series of unproductive discussions?
- the world is going to run on software, even more so than now, how do we make sure that software is doing what we want?
- (hint: agents are just software, too)
- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale

---

# Why

- the history of programming is one of monotonically increasing levels of abstraction, from handcrafted gears to wiring up vacuum tubes to punch cards to machine code to assembly to C to Java to TypeScript
- climbing the ladder of abstraction generally yields more output for every bit of effort, as we remove ourselves from the annoying burden of having to actually spell out what _exactly_ it is we want the machine to be doing (which bits? which registers? what memory? which network? what computer? what archictecture? ... etc.)
- it follows then, that we might want to go all the way, that the ultimate sophistication of programming is not programming at all, but more like "talking to a colleague"? 
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple and unconstraint as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by formal languages

- we already have precedent to understand what that multimodal, low-code complex engineering might look like: game development!
- the future of software development is lots more like game development
- parallels to the early personal software revolution as well
- early on, everyone has to wire up the pieces themselves, there are few standards, everyone is handrolling OSes and floppy drivers or whatever
- then higher level standardized systems emerged, not perfect, maybe you didn't always get exactly the same level of control, but it was a lot more productive
- sometimes you write engine, sometimes you write scripts
- lot of the time is just iterating in some more interactive editor
- sometimes you "play" the game, sometimes you pop out to edit the game
- lots of internal tools and proprietary

## Why "Code-First"

- "coding is solved" ("bugs not solved yet")
- why should we even think about code at all if it can all be written by AI anyway
- what is the point of code? why did we ever write any code? 
- or why think about numbers when calculators exist
- it's not _just_ "lossy abstractions" a la "why think about how assembly works when you can just write Python", it's also a difference in _kind_

- programming is fundamentally about problem solving
- however, the contemporary theme seems to be in _removing_ me from the details, and "just have humans give high level direction"
- I've tried that, it doesn't work, I don't want to do that
- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.
- I want to know, what is the space of all possible software programs to solve my program, and how do I most efficiently get there, where do I go when I'm there, how do I stay in the right place, and so on
- we used to solve problems by a manual next-character-predictor with a keyword, now we can often work at a higher level.
- but we must still understand, or the software sits in some weird disconnected castle in the sky that serves nobody
- (well over a year of "vibe coding" has shown this pretty concolusively)

- nobody really wants _code_ much like nobody wants _software_ or _computers_ in and of themselves
- the code is not the product, and usually the software is not the product, either
- idea of just generating bytecode _directly and only_ is fanciful and basically a meme
- it makes very little sense, why would we waste tokens on less maintainable code with fewer guarantees
- there is a reason humans have evolved higher level languages, and while we may not settle at the exact same level of abstraction (indeed, TS++ draws the lines a little differently), it will almost certainly not be "generate machine code directly" for general purpose software
- as long as there are humans involved at the edges and on the sign off, we need some common ground

- it follows that dropping code, doing something higher level, this sorta kinda works, sometimes, actually!
- spreadsheets have worked for decades, game dev people have been doing this for a while, we have now figured out a new way to prompt simple software into existence
- (historically, when some more "accessible" programming-ish becomes too common, the "real" programmers no longer consider it programming. thus, excel is not "programming", just like image classification is not "AI")
- and of course, ther eis the trivial observation that we're _already_ not writing most code ourselves - the OS, standard libraries, dependency ecosystems, some compiler/transpiler is writing the actual low level code for us, ..

- the trouble is, in *some* software, there is no meaningful separation of the system and its specification, - and thus there is no magic abstraction on top of code that will solve _all_ our problems - if the job is solving novel problems.
- (the granularity of the specification to care about depends strongly on how standardized the solution is. fine tuning a character controller? better control every bit of entropy. building an email sender? just plop in a framework, we already do this.)
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, "APIs will replace everything", etc.)
- abstracting coding away is in itself a "lossy abstraction"

## Why "Human-First"

- there is a popular current of "agent first design", and a bunch of developer-adjacent tools are being rebuilt to become "agent native".
- I don't really know what that means, beyond good APIs, high performance, and .. just building software in the way we should have done anyway.
<!--- "agent native" makes for good marketing and pitch decks, but means little in practice-->
- most things that are "good for agents" - fast iteration, clean boundaries, programmable software - are good for humans too, we just haven't had the opportunity to the big rewrite until now
<!--- and there also just hasn't been a good opportunity to reconsider deepset habits yet -->

- fundamentally, there is not a single test, suite of tests, mathematical proof, or specific gate that you can run to convince me that some non-trivial program is correct
- doesn't matter whether it's human written or not, software is just very complex
- correctness = alignment + visibility
- correctness is ultimately about alignment, and we can only align on what we can see
- I don't know what I want until I see it, and I also don't know what I _don't_ want until I see that too
- correctness is an iterative process, alignment is continuous, the shape is changing
- "correctness" must be specified acrosss many layers to systematically exclude all the things we do _not_ want
- (and ofc there are probabilistic assessments that are even harder to nail down)

- our tools for writing, interacting with, understanding software are pretty primitive
- if everything is code, how do we make sure it's the right code?
- "oh just have the AI tell you if the code is right" but again what is right?
- (in this sense, the "alignment problem" feels much more like a product and legibility problem, and certainly not _merely_ an intelligence problem, which is short term bearish but long term very bullish)
- this does not magically go away with more abstractions or "smarter AI"
- understand the shape of software and the space of all possible software
- make a map
- much better static and dynamic analysis
- "software in motion"
- code is going to run _everything_, even more so than it already does (literally)

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

## Why "Complete"

- there is something beautiful about doing the most with the fewest possible parts
- a minimal, simple language like C or even Go - though very few people would have called either "minimal" at the time they were introduced - is elegant in a way.
- it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases
- the carcinisation of (managed) languages
- however, over time, most serious languages with actual production use evolve an set of common features for building serious software
- Go and generics, Java / C# and unsafe / structs / ref, ...
- JVM/CLR by default, Rust on demand

- hardware is getting *more* expensive
<!--- we're going to get a lot more software-->
- simulating software is 

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
- bigint and string as lowercase primitives are fine, not ideal, but fine

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
- not necessarily "what is the best theoretical version of something like typescript" (default decision = remove) but "what is the most typescript we can make it, removing only what is absolutely necessary" (default decision = keep)
- safe, sound, predictable

- TypeScript is tantalizingly close to being a serious, native, _universal_ programming language
- Could we get something _like_ TypeScript to run like a "serious managed" language a la JVM or CLR or Go
- (AssemblyScript and friends fail in the 'feel like TS' department, and Static Hermes does not by design attempt to go "beyond" TS either, which means we need to start from scratch)
- the dichotomy between "scripting languages" and "systems languages" no longer makes much sense if it's not humans doing the typing (assuming "compile times" are fast)
- begone with the need for a separate language and stack to run "backend" or "compute heavy" tasks once we "outgrow" node.js or whatever.
- .. TypeScript also happens to be the very same language that runs the web, the biggest software platform in the world!
- but of course, it would have to really *feel* like TypeScript, not just "look" like TypeScript! as much as possible, TypeScript semantics - far beyond the surface syntax - should be preserved for this to really be a day one language.

- what's the most boring thing we could build?
- it's a little weird, but all we really need is familiar TS ergonomics with reliable Rust performance 
- mechnically, what is the ergonomic ladder of TS++ between TS -> Rust
- can that even work?
- needs to be a stable foundation for all the other crazier stuff
- don't try to be cute or clever or fancy
- no "improvements", only corrections
- don't "fix" what's not _actually_ broken
- only use boring ideas already proven by other languages / libraries / ..

- TS++ fashions itself as a "superset of a strict subset of TS", which is vaguely reminiscient of the relationship between C and C++
- the obvious first cut is to remove any express soundness holes from TS
- then we figure out how to map the type system to something we can actually compile for real
- and what we need to add to the types and expressions and runtime to make this a serious systems language
- the procedure then is not one of starting with zero, it is minimally subtractive:
- what do we _need_ to remove because it is unsound / legacy / strictly superseded by something clearly better?

---

## Types

- types are great, strict type systems are fantastic
- I actually like the TS type system for the most part
- we just need to make it strict and sound
- remove some footguns, remove weirdness
- we get to keep muscle memory

- and ofc need to be stable and deterministic, incrementally compilable, strict boundaries, make it fast
- cover as much as possible with type algebra
- the interesting question is how much of TS can we make sound, predictable, and fast.
- quite a lot, actually
- and it turns out that TS algebra and generics are much faster than macros since it's essentially a very constrained macro system already
- (though we still plan on having a more proper compile time macro system as well based on `eval`)

### Soundness

- the easy part first to first: kill all the no soundness holes!
- no dynamic shenanigans, no JS legacy compat
- need strict, sound TS with predictable module boundaries and type behavior (roughly equivalent to tsc's `isolatedDeclarations`)
- changing and constraining the type system just a bit gives us a lot more parallelism
- ideally also make it fast to compile, which requires cleaner boundaries than standard TS gives

All of the inherited JavaScript legacy-era dynamisms must go:
- no dynamic JS shenanigans or monkey patching, so goodbye to `__proto__` or anything like that
- no `module.x = foo..`
- no `Object.isOwnProperty`, `Object.assign`, ...
- no "truthiness"; conditionals always take booleans
- no array holes
- no sequence expressions, who needs sequence expressions

There are also some TypeScript features that are not sound or just not needed in a purely strict model:
- no declaration merging / no separate type and value spaces (as such)
- no `any`, no `as` except for widening casts
- no predicate functions (e.g. `isUser(user: any): asserts user is User` is unsound)
- unknown still works as a fat existential
- no symbol / string keyed duck typing (proper traits)

### Primitives

- number, yes, but int32, uint32, int64, float32, character too
- no real symbol use case left, so no `symbol` or `unique symbol`
- string and bigint are just regular classes (String, BigInt)
- variable sized integers
- isize / usize
- (sequence collections default to isize instead of number)
- keep null and undefined, no strong reason not to

- keep freshness and widening
- keep literal freshness
- const / as const

### Enums

- enums are reasonably simple
- no const enum needed?
- integer and string..?
- string enums are just managed strings
- auto incrementing enum (starts at 0, int64, signed)
- enums are nominal! need to explicitly cast

### Arrays, Slices and Tuples

- tuples, slices, inline arrays and the rest
- no more array tuples (need to free up `[T]` and `[T; N]`, arbitrary `[X, Y, ...]` is an error)
- slices are just fat pointers
- `&[T]` is also a fat pointer, `[T]` is a managed slice, `^[T]` is an owned slice
- fixed arrays `[T; N]`
- `[T]` is managed by default (just like `Function` and `Dynamic` are managed fat pointers by default)

### Classes

- classes are generally pretty straightforward, it's just about which tradeoffs do we want?
- about a dozen way of doing classes, from decent to okay to weird JVM? C++? Go?
- "zero overhead"? vtable pointers? explicit or implicit virtual?
- allocation metadata sidetable on the heap / runtime
- classes are reference types by default, alias freely
- abstract, final classes
- virtual, override methods

- in TS, like in many managed languages, we can just omit the "self" parameter in a method and the receiver will just default to the aliasing managed reference "this"
- we support this ofc as well:
- `this` = `Managed<T>` for reference types
- implicit and explicit this
- value and borrowed forms

- no method binding (i.e. no sneaky obj.method, instead use () => object.method()) for clarity)

- visibility:
- default is public (as in TS)
- private/protected is per module, not per item
- no need for #privateField
- we have private, we just use that
- and it codegens to #privateField on JS targets
- no additional visibiliity controls
- 
### Structs

- every serious programming language eventually cares about memory layout and allocations
- need fixed no overhead shapes
- no inheritance
- no embedding (unlike Go, Jai)
- no constructors, getters or setters

- `this` = `&exclusive T` for value types (more on that soon)

### Newtypes

- usually use symbol branding in TS, which is kinda icky

```ts
const Tag: unique symbol = Symbol();

{
    [Symbol.toStringTag]: () => "Tagged"
}
```

- proper nominality and newtypes
- newtype, newtype interfaces (traits)

```ds
newtype UserId = string;
newtype UserId = private string; // can only be constructed within file
```

```ds
newtype interface Add<T> {
    add(a: T, b: T): T;
}
```

### Algebra

- TS algebra is actually great for doing light metaprogramming
- want to preserve as much of it as possible
- thankfully, it's actually mostly sound already
- especially once we agree that structural types are exact / fixed, and interfaces are dynamic

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`?
- (no, has to match exactly, in order)
- but you can pass it to `Dynamic<Point>` or `interface Point` (same thing)

- interval types over finite sets (integers)

- deep readonly
- const is *not* readonly (just like in TS)
- const is however overwrite stable

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
- `Record` is read-only (type Record<K, V> = { readonly [K]: V })

```ds
export type Record<K: PropertyKey, V> = {
    [P in K]: V;
};
```

- funky signatures:
- call signatures (just a `Function` fat pointer)
- construct signatures (also a `Function` fat pointer)
- index signatures
    - can I read through index signatures? can I call through them?
    - (index signatures use `dynamic.find` at runtime, which is a linear scan over string equality!)

### Generics

- trivial syntactic cleanup: `T extends string` -> `T: string`
- stay the same basically
- in, out, in out, measured variance
- generalised `const` parameter for value generics (literal types with a fixed cardinality of one, measured by usage like with variance)

- monomorph or not to monomorph
- how far? do we monomorph refs?
- JVM / CLR -> Go -> Rust / C++
- build vs release mode

- where clauses
- where clauses on members and extensions
- any (trivially) statically decidable predicate
- `foo<const T: isize>() where T > 5`
- measured cardinality (like measured variance)

### Narrowing

- type narrowing as usual, narrowing is just doing runtime type checking
- instanceof, typeof, is
- typeof in type position
- is for type queries
- instanceof for classes
- match narrowing

---

## Expressions

- keep all the ergonomics and muscle memory
- remove some legacy weirdness
- "expressions as values"
- errors as values (Result)
- pattern matching
- (no weird switch fallthrough)

### TSX

- tag based trees are pretty useful and broadly applicable
- there are other ways of doing UI, but this is a pretty good one, and it's *very* familiar
- trees, generalised tree litearls,
- lowercase tree builders, ..?
- support both "elements" and "fragments"
- (unfortunately this also means keeping TS ambiguity around..)

- contextual TreeBuilder interface incl. string tags

### Patterns and Match

- patterns
- refutable vs irrefutable
- match
- `Sequence` type
- ... except for dynamic index signatures where it types as `V | undefined` (via dynamic.find)
- ranges: `..`
- switch still works but match encouraged

- let, let-else
- if let
- while let
- match guards

- No Computed Keys
- no `obj[expr]` where `expr` is dynamic
- destructure dynamically with `{ [key]: value }`?

### Decorators

- JS/TS already sorta kinda has decorators, sometimes
- extended placement
- decorators on expressions
- newtypes as decorators
- incl. union newtypes

- queryable
- `@if` static gating
- taint/tag system

### Documentation

- builtin ish?
- cargo doc?
- jsdoc?
- documentations on all expressions (like decorators)

### Result and Try

- Most subtractions and additions between from TS++ to TS are about soundness, but there is nothing intrinsically unsound about exceptions. 
- if there is one really bad error in modern managed languages, it's exceptions
- this one is a little more subjective, but given the pain caused.. exceptions most die. we cannot have an invisible side channel infecting everything in the last computing stack
- checked exceptions are even worse

- honestly, nobody has figured out a *great* way to do error handling (looking at Go here in particular), but Swift and Rust's Result-shaped error values with Try operators `?` are pretty good
- panic/unwind still exists like in rust but that's worker-scoped, not normal recovery
- Try operator, ? ambiguity because TS
- but exceptions have proven troubling over and over and over again
- checked exceptions are even worse
- the only sane error handling method is the Swift-y Rust-y ? operator 

- result and async (promise / task)

- Try-Catch-Finally still works
- familiar try / catch / finally syntax still works though!
- catch (e) is all Try error residuals
- catch match (e) as the ergonomic switch

### Using

- using / async using (like TC39 proposal)
- Dispose / AsyncDIspose

- vs Drop

### Extensions

- Rust has `impl` blocks for as the _sole_ mechanism for attaching members to nominal targets
- TS++ has its as an additional mechanism
- like `impl` in Rust but a little broader
- inherent, anonymous, named extensions
- E / T, T may be local or imported
- `extension of T`
- `extension E of T`
- `export extension of T`
- `export extension E of T`

- `extension<T> of T`: blanket extension
- rustc coherence
- no orphan rule? 
- global extensions considered for impls (not import order)
- member overloading only within a single declaration block (extension or itme declaration)
- also for @unsafe impls

### Operator Overloading

- serious math-y applications want operator overloading
- newtype interfaces ("traits")
- binary `Add`, `Subtract`, `Multiply`, `Divide`, etc.
- unary `Plus`, `Minus`
- `Vector2<float32> + Vector2<float32>`

### Const

- originally envisioned something closer to Zig's comptime (or even Jai's version of it)
- originally had a comptime keyword here but was kinda confusing
- `const <expr>` and `const { ... }` for comptime evaluation
- `const function` for comptime functions that can only be called at comptile time
- cardinality is measured by usage (sort of like how variance and )

### Functions, Lambdas and Captures

- lambdas (fat pointers with env)
- `Function`, `^Function`, `&Function`
- multiplicity "once" and "repeat"
- `FunctionPointer` for raw function pointers without environment

- `@capture`

### Async

- proper async
- keep familiar Promise for aliased async
- introduce Task for structured affine concurrency (same async/await model)
- (Promise = managed class, Task = value type, Promise requires aliasable / copyable type)

- *fiber*-based execution (e.g. JVM's new model)

### Context and ContextVars

- like Python
- but for all bindings
- `Context`

### Panic

- overflows / underflows
- out of bounds
- deliberate unreachable
- worker scoped
- catch unwind

---

## Memory

- TypeScript, following JS, has no real way to control memory shapes or allocations directly
- of course, most serious web programmers think about hidden class caches and all the brilliantly engineered details of the popular JS engines to keep their software reasonably fast
- (asm.js, yes, but, no.)
- we can trivially restrict to closed shapes, which buys us predictable layouts

- fixed static shapes (post type algebra solving) .. that's already fine for 80% of use cases, basically what C# / JVM / Go-ish are
- but sometimes we want even more: proper value types, borrowing, pointers (gasp)

### Representation

- so, TS++ should behave as much as TS as we can physically manage while keeping sane and predictable performance _and_ behavior
- (and something we can actually build into a good toolchain)
- by default follows rust-y layout, including niche optimisation
- null / undefined / nullish is stored in the same address (currently bit patterns just 0x0 and 0x1)

- @repr
- custom repr
- @repr("C")

### Ownership

- Value Types, wooo
- with move semantics
- bare T just means whatever the default form is. preserve TS behavior
- reference types are reference types, value types are value types
- ^T, T, &T, *T, ...
- Managed<T>, Owned<T>, ...

- managed vs owned bridge
- classes are managed by default
- even when the rvalue is owwned
- this is to preserve the key feeling of e.g. Arrays and such, while enabling full owned no-managed where desired

- references are safe and sound, statically guaranteed 
- sometimes we need stuff that cannot be statically proven.. raw pointers, *T
- arrghh yes seriously pointers in TypeScript let's go

### Borrowing

- if we want value types and we want to pass them around, we need some way to reference them safely
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, so if all types can contain references, just like in Rust, ... we need proper borrowing rules

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- generalised lifetimes into regions (combine lifetime + space/place)
- T & 'a, 'a & "shared", ...
- Borrowed<T, L/R, A>, REadonlyBorrowed, ExclusiveBorrowed
- tried a bunch of things to make this more TS-native, but ultimately, the Rust model really is best (inference only locally within functions, no induced generics beyond that)

### Mutability

- let / const preserve TS meaning
- can take &exclusive only on managed types for const

- readonly
- &T default to mutable
- &readonly for explicit readonly
- Rust only has mutable vs immutable
- "third rung" on the mutability ladder
- overwrite stability

- we can now distinguish "readonly, non-exclusive", "mutable, non-exclusive", "mutable, exclusive"
- (what about data races..? lints / DST / ...)
- exclusive ownership
- worker-local, borrowing

- WithAccess
- PlaceOf
- ...

### Drop

- Drop is *eager* (unlike in Rust)
- Drop is for "infallible" memory management, using is for actual resources
- Drop also runs as a "finaliser" 
- (e.g. Drop on an Array deallocates the memory)
- no drop flags needed because no partial initialisation + eager drop
- Drop is *not* lowered to JS (not sure how that would even work..?)

- it's a finalizer, but a very restricted one
- no allocations, no panics, statically checked

### Local and Shared

- so far we have assumed basically single-threaded, async execution
- this is most code, but obviously a complete language needs to consider concurrency at a more fundamental level, across threads
- many ways to do this, TS already strongly biases into the "local-first" direction
- we could just generalise SharedArrayBuffer and friends?
- split local and shared memory spaces
- separate heaps, separate GCs
- worker-first, local-first, shared-nothing-first memory model

- local isolated heap per worker
- local and shared modifier on types
- local and shared modifier on bindings
- local and shared modifier on declarations
- worker-local stuff is .. local (Promise, Task, etc.)
- no need for Send and Sync, basically the 90 degree rotated version of that classic pair

- local borrowing managed is sound except across suspension
- how to keep local / shared safe
- proper managed object types on shared

---

## Runtime

- what is the perfect runtime? dunno. probably doesn't really matter anymore.
- again keep conceptual muscle memory
- as with everything else try to keep as familiar as possible

- Burning the Boats
- No Backward Compatibility
- first and most serious cut is to drop support for existing .ts/.tsx (and .js/.jsx) alltogether
- no NPM, no JS bridge, no TS "best effort", no fallbacks, no compatibility bridge, nada.
- standardization, integration, .. the whole thing only works with a blank slate
- so, decidedly not "byte identical"
- .. but basically feels the same


- "Write Once, Run Everywhere"
- yada yada heard it a million times
- (though it did arguably sorta work for Java, and now the web , and maybe WASM, .. mostly)
- want portable
- always build from source?

### ESM Modules

- strictly ESM imports and exports
- imports, exports, re-exports, defaults, etc. it's all the same
- import data files 
- no async imports / exports
- no CommonJS
- no export type / import type

### Workers

- JS/TS already has a strong worker-first story
- already talked about local / shared heap split
- shared memory across workers, local heap to each worker 
- (this is fortunate because it gives us the local managed/borrowed model we want)
- retain local / worker isolation as the primary model
- use Workers for structured concurrency
- (maps to threads N:M)

### destack.json

- oh what is the theoretically ideally package format? toml? json? txt? CMakeLists? magic setup.py? just kidding
- package.json is okay, let's just use that -> destack.json
- json based, json is nice, let's use use that
- familiar mental model, just combine the disparate pieces
- combine electron, expo, package.json, Cargo.toml, ...

```json
{
  "$schema": "https://destack.sh/schemas/destack.schema.json",
  "name": "my-destack-project",
  "version": "0.1.2",
  "private": true,
  "targets": {
    "default": {
      "include": ["src/**/*.ds"],
      "output": "program"
    }
  },
  "defaultTarget": "default"
}
```

- conditions:
- @if static for expression/item-level gating
- Generalised to Modules with "conditions"
- x.ds, x.test.ds, x.whatever.ds
- gate on import.meta

### import.meta

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string \| undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string \| undefined` | `"/app/src"`, `undefined` |
| `import.meta.output` | output artifact | `Output` | `"bundle"`, `"program"` |
| `import.meta.platform` | target operating system | `Platform` | `"linux"`, `"windows"`, `"none"` |
| `import.meta.host` | target host environment | `Host` | `"browser"`, `"native"`, `"wasi"` |
| `import.meta.target` | target family and ABI | `Target` | `{ family: "unix", arch: "x64", abi: "gnu" }` |
| `import.meta.targetName` | active build target name | `string \| undefined` | `"web"`, `"native"` |
| `import.meta.product` | active deliverable product name | `Product \| undefined` | `"app"`, `"server"` |
| `import.meta.version` | active package version | `string \| undefined` | `"2026.5.2"` |
| `import.meta.stability` | active package or product stability | `Stability \| undefined` | `"alpha"`, `"stable"` |
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.<mode>` | mode shorthands for `debug`, `dev`, `prod`, `test`, `bench`, `lint` | `boolean` | `import.meta.test`, `import.meta.prod` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string \| undefined }` | `{ NODE_ENV: "production" }` |

### Bindings

- proper colored functions
- stdlib based on explicit @bindings
- effect tracking
- @binding
- configured via Context

<!--### Style

- one of the perks of owning the whole toolchain is we can make the parser and formatter and linter and such do whatever we want
- doc comments on expressions too, why not
- colored regions?
- logic blocks
- neurotic code styles
- annoyingly pedantic linting (but it's fine)-->

<!--### Topology

- Entity, Edge, ...
- C4, ...-->

<!--### Testing

- destack:test
- jest / vitest style
- again, sticking to good prior art-->

<!--### Standard Library

- again: what is the most widely known, idiomatic shape here that everyone already knows?
- in this case: Node and Web shaped
- `node:*` -> `destack:*`
- Web* APIs
- fetch, .. whatever good standards the Web has
- Rust-y stdlib underneath-->

<!--### Linter-->
