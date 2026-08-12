---
title: "introducing typescript++"
subtitle: "evolving TypeScript into the last programming language"
date: "2026-08-18"
tags: ["language", "runtime", "platform"]
author: "Florian"
---

# introduction

- TypeScript is tantalizingly close to being a serious, natively compilable programming language
- (this shouldn't be _that_ surprising considering its forebearer was loosely based and slowly evolved next to Java, and ofc course its creator is _also_ the author of C#, itself a very respectable and pretty fast language)
- we would like typescript to run "native" very much, since that would mean we can finally have predictable systems-ish performance out of the same language we already like and that runs the web, which happens to be the biggest software platform in the world

- we live in an age of miracles, and you can now actually ask your computer to build a typescript compiler *for you*! 
- and it will go off and after a few hours it will (hopefully) come back with something that maybe kinda sorta works! we didn't learn anything in the process, and it's not entirely clear what to do with 1M lines of unaudited C code (the LLM chose C because that's easier to bootstrap or something), but the tests are green! and after some 

- human-first design
- curious trend of "agent native" programming tools
- upon closer inspection, it is never quite clear what exactly makes some piece of software or infrastructure more "agent native" than something engineered for, say, mere humans
- doesn't really mean anything
- "agent native" mostly seems to mean "churning out as much code as possible while delegating review to even more agents", which is the exact opposite of what I want
- (relatedly, "how to manage dozens of agents" is not a problem I experience nor does Destack do anything _specific_ to "solve" that "problem")

- common theme seems to be in _removing_ me from the details, and "just have humans give high level direction"
- I've tried that, it doesn't work, I don't want to do that
- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.

- tools for those who still care
- on one hand, I'm lazy, and don't want to learn new stuff unless absolutely necessary (or really interesting)
- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale

# why

## why even bother with programming languages

- just as we have long tried to rise higher up the ladder of abstraction in code, we have also tried to somehow remove ourselves from the troubling burden of having to actually spell out what exactly it is we want the machine to be doing
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by

- the trouble is, in software, there is no meaningful separation of the system and its specification
- there is no magic abstraction on top of code that will solve all our problems
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, etc.)
- now, many of these higher level specifications have legitimate use cases

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

## why do we need a universal language

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

## why combine typescript and rust

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

## why stay within the lines

- why not extend what already exists
- static hermes, assembly script, ...

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

## why now

- up until less than a year ago, seriously proposing new languages and software ecosystems would have been insane
- it took years of iteration and development for Rust, Mojo, Zig, .. to get off the ground and reach respectable levels of maturity and adoption
- having a language that looks like typescript but doesn't directly run most existing typescript appears to be an odd positioning; however, the goal is not to be a 1:1 mapping since it's impossible to "just run TS" without making significant tradeoffs in either direction. 
- "one-shot portable"
- (should be locally portable by going file by file without global context)

- thousands of supply chain attacks in waiting
- explicit policy controls per package
- vendoring encouraged for smaller dependencies (shadcn registry style over big bowl of dependencies)

# how

- what is the minimum set of changes / additions we need to good prior art to get what we need
- safe, sound, predictable
- about two dozen or so key decisions to be made when building "typescript++"
- how dynamic do we want to support?
- do we want to support "escape hatches"? 
- any sort of backward compatibility

## no backward compatibility

- first and most serious cut is to drop support for existing .ts/.tsx

## obviously, no <soundness hole>

- any
- sneaky casts
- JS shenanigans
- array holes
- no predicate functions (e.g. `isUser(user: any): user is User` is unsound)

## strictest TS

- need strict sound TS with predictable module boundaries and type behavior
- isolatedDeclarations
- strictNullChecks
- strictFunctionTypes
- strictPropertyInitialization
- no "truthiness", conditionals always take booleans
- ...

## ESM modules

- strictly ESM imports and exports
- no CommonJS

## constrained dynamic (?)

- shape mutation
- excess properties
- declaration exprsesions
- dynamic prototypes
- all sorts of JS hacks that everyone hates anyway

## proper primitives

- number, yes, but int32, int64, float32, character too
- tuples slices inline arrays and the rest

## no exceptions, results only

- most subjective of the bunch
- but exceptions have proven troubling over and over and over again
- checked exceptions are even worse
- the only sane error handling method is the Swift-y Rust-y ? operator 

## patterns

- patterns
- match
- catch match

## decorators

- extended placement
- newtypes as decorators

## nominality

- usually use symbol branding in TS, which is kinda icky
- proper nominality and newtypes
- newtype traits

## operator overloading

- serious math-y applications want operator overloading
- `Add`, `Subtract`, `Multiply`, `Divide`, etc.

## object types, but how

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`?

## classes, yes, but which ones

- zero overhead? vtable pointers?

## `Dynamic` and structural interfaces

- how dynamic do we want to go
- index signatures
- can I read through index signatures? can I call through them?
- call signatures

## value types

- every serious programming language eventually cares about memory layout
- need fixed no overhead shapes

## generics and variance

- stay the same basically
- new `comptime` parameter for value generics

## ownership

- ^T, T, &T, *T, ...

## borrowing

- if we want value types and we want to pass them around, we need some form of borrowing
- we *could* do this asthe C# way and have in / inout / out style params, which is half the solution
- but we want to be unviversal, and we want ot be safe, 

## lifetimes

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- tried a bunch of things to make this more TS-native, but ultimately, the Rust model really is best (inference only locally within functions, no induced generics beyond that)

## access, mutability, exclusive

- readonly
- &T default to mutable
- exclusive ownership
- worker-local, borrowing

## local and shared memory spaces

- SharedArrayBuffer and friends?
- worker-first, local-first, shared-nothing-first memory model

## async, promise, tasks

- proper async
- keep Promise for aliased async
- introduce Task for structured affine concurrency (same async/await model)
- fiber-based execution (e.g. JVM's new model)

## panics, traps

- overflows / underflows
- out of bounds
- deliberate unreachable
