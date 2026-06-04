---
title: "introducing destack"
subtitle: "the absurdly integrated stack for humans building correct, optimal, integrated software systems on TypeScript++"
date: "2026-06-14"
tags: ["language", "runtime", "platform"]
author: "Florian"
---

In 1972, [C introduced higher order programming](https://en.wikipedia.org/wiki/C_(programming_language)#History), and some 50 years later, programming is still astoundingly immature.
Software "engineering" is anything but, and while our tools have gotten prettier, the fundamental motions are unchanged.
Despite incredible hardware advances, accumulated wisdom, and many libraries and languages and tools, nobody today would accuse the average software product of being anywhere close to correct or optimal.

- curious state of affairs that software, the _one_ domain where we should be able to draw and redraw new lines of abstraction quite freely, is so stuck on a specific evolved path
- the 30 million lines problem
- dependency and supply chain risks
- software should be deterministic, predictable, simulatable, but it's anything but

- for some reason, the "stack" has become a thingy worthy of worship on its own, as though the mere breadth and tools required to do a job has any bearing on the quality of the result
- it's not uncommon of course to have professionals take pride in the tools they choose and develop some affinity for specific
- having a starter kit
- sure, other industries have vendors as well, and there's nothing inherently wrong with that, but it's a little strange we can't even run most software locally properly without leaky mocks

## higher-order programming

- there is a long running trend in the history of software of rising abstractions
- more abstraction is generally considered a good thing, removing the need to think aboiut those pesky "details of the machine" and focus on the "problem at hand" (a variant of the much cited "focus on what makes your beer test better" quote)
- (it should be noted to the attentative reader, that we all implicitly know that the best programmers know a ton about the lowest levels of detail, even though they may choose to work higher up the stack)
- increasing levels of abstraction along multiple dimensions:

-  language: from punch cards, to machine code, to assembly code, to autoconf / C, to "managed" languages
-  it should be noted that the only reason this works (for some definition of "works" anyway), is that these are precise, symbolic translations
- millions of engineering hours have been spent on optimizing these layers to make it possible to e.g. write JavaScript and have it execute in some reasonable way on modern hardware without any regard for the reality of the machine
- now, you may not like this, but it is impressive.
- certainly, on the other end, languages like Rust and to some extent "modern" C++ have come a log way in offering "higher level features" at "zero cost" abstractions

- libraries: object orientation and frameworks and CRUD (Ruby / Rails, Django, ...)
- it's genuinely great to reuse work, and nothing would ever move forward if everyone had to rewrite _everything_ from scratch, every time (and nothing would work properly together, either)
- sometimes you don't know how exactly to do something, and using a library is a sort of scaffolding to the solution of your problem that you can buy into
- someone else already has figured out the problem for you - not only do you get a physics engine, you get the domain model to think about physics engines and can leverage that in your own application
- (and ideally the language / library / compiler will guide you into using it correctly)

- services: there's a third curious dimension along which software has grown, and it has grown a lot: services. it's almost impossible to ship commercial software without talking to at least a handful of other service providers, and thus your own
- not just "the cloud", but also ctual third party services that your application _must_ call out to
- it's not uncommon to have multi-page secrets / tokens files that have 10+ entries for connecting to different services, any one of which may take down various aspects of your software at any moment without advance notice

- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale


## the system is the specification

- just as we have long tried to rise higher up the ladder of abstraction in code, we have also tried to somehow remove ourselves from the troubling burden of having to actually spell out what exactly it is we want the machine to be doing
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by

- the trouble is, in software, there is no meaningful separation of the system and its specification
- there is no magic abstraction on top of code that will solve our problems
- every attempt to put something "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, etc.)
- now, some of these higher level specifications have legitimate use cases

- programming is somewhat unique in this regard because in other domains.. if you have a sufficiently detailed specification - i.e. a building plan - for a bridge you're going to construct, there should be absolutely no surprise when it's actually built and you can hand it out to any competent contractor for the actual act of construction.
- code seems to be curiously different in that we often don't really have a clear specification upfront, and establishing that specification _is_ equivalent to building the system
- the code is the ultimate specification
- whether it's the tests, the system behavior, some design document, but ultimately, we just haven't figured out any real way of splitting out the "specification" from the system, because the specification _is_ the system.

- a sufficiently detailed specification that has no ambiguity in its execution _is_, by definition, executable
- .. even though it may not literally be code .. but it usually ends up looking like code
- .. the dream of "if only we had something less harsh than symbolic systems / code" is old and fails every time
- even if we could magically

## correctness = alignment + visibility

- there is not a single test, or suite of tests, mathematical proof, or specific gate that you can run to convince me that some non-trivial program is correct (irrespective of human written or not)
- correctness is ultimately about alignment, and we can only align on what we can see
- I don't know what I want until I see it, and I also don'T know what I _don't_ want until I see it

- correctness is iterative, the shape is changing
- correctness must be specified acrosss many layers to systematically exclude all the things we do _not_ want
- the reality of the system must be naked and familiar

- incremental precision
- who measures the measurer? how do we know that the 1kg calibration stone is really exactly 1kg?
- alignment requires precision
- precision must be built on a solid foundation, incrementally
- precision requires looking at the code, systems
- .. from many angles, in detail, high low, in motion, statically, all sorts of dynamics, ...
- incremental granularity (a la casey muratori)
- you can't engineer precision and alignment (i.e. understanding) into a system post-hoc (or at least, only with great difficulty that far exceeds the cost of doing it properly from the start)

## one language, one toolchain, one stack

- why integration?
- historical lines that make sense as the markers of evolved sedimentary layers
- but we now understand what they are and ca nconfidently redraw them - with a light touch
- it's all one product
- tomorrow tech demo

- constraints and state spaces
- well known topology
- engineering and understanding systems
- correctness and optimality
- strong "pre-deploy" guarantees including but not limited to strong typestate patterns, compiler checks, linters, .. custom lints, custom static and dynamic analysis, software in motion, DST, ...
- DST! benches! fuzzing! reflection! they all relate!

- policy, permissioning, deny-by-default, zero-trust
- granularity and multi level views
- need to be in the details
- birds eye, tracer bullet, slow motion, etc...

## human-first design

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

## typescript++

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
 - runs natively at machine speed

- why a new language
- why fuse typescript and rust
- why a universal language
- why now

- but alas, hardware is real, and if we want to make fast software, we need to control those low level pesky details somehow
- typescript is tantalizingly close to a systems language
- remove all the dynamic / JS baggage, add a little bit of layout and memory control, and we're looking at a surprisingly presentable low level language

## optimality requires expressivity

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

- "second system effect"
- "why not fix all the problems"
- "boiling the ocean"
- the "ideal system"
- if we can port to whatever we want, why not do something entirely new?
- safety, safety, and "safety"
- aliasing mutable borrows, the loss of a bit of entropy with `&T`

## homoiconicity -> hackability

- sim-to-real begone
- we have the most simulatable medium of any industry and yet we can't freaking manage to be sure that "send email" will work in prod
- it's a freaking circus for no real reason
- an ecosystem with simulation built in

- single-process systems
- can you ship your software as a single executable?
- in a more basic sense, does it even work offline? or does it just panic?

- can't hack what you can't see (not really)
- fully open source from the ground up
- fully integrated to an absurd degree
- one standard way of doing standard things (logging, telemetry, errors, UI, ...)
- custom software on strong standard foundations
- let a billion baby apps bloom

## bootstrapping an ecosystem

- up until less than a year ago, seriously proposing new languages and software ecosystems would have been insane
- it took years of iteration and development for Rust, Mojo, Zig, .. to get off the ground and reach respectable levels of maturity and adoption
- having a language that looks like typescript but doesn't directly run most existing typescript appears to be an odd positioning; however, the goal is not to be a 1:1 mapping since it's impossible to "just run TS" without making significant tradeoffs in either direction. 
- "one-shot portable"
- (should be locally portable by going file by file without global context)

- thousands of supply chain attacks in waiting
- explicit policy controls per package
- vendoring encouraged for smaller dependencies (shadcn registry style over big bowl of dependencies)
