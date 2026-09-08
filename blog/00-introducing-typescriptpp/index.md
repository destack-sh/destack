---
title: "Introducing TypeScript++"
subtitle: "Evolving TypeScript into the Last Programming Language"
date: "2026-09-14"
author: "Florian"
---

- we're about to enter a cambrian explosion of software
- this is great, except that it's is worth remembering that at the end of that period, most animals and plants had died, and the surviving species looked rather different from what had been dominant before.

- it's not hard to see how the status quo of software - and its production process - kinda sucks.
- we have evolved a slow artisinal process with uncertain non-repeatable step: 
- dependency sprawl
- uncertain timelines
- supply chain attacks
- and if lucky, at the end of this process, we are rewarded with slow, buggy, clunky software!
<!--- very hard to run software locally-->

[illustration of cambrian explosion carnage](TODO)

- we're not going to get _great_ software by averaging past software, or by merely "accelerating"  existing software production processes. 
- we need _new_ processes
- so, what does great software look like, and what does great software _production_ look like?

<!--- and _now_ we're asking even _more_ from our software than ever before-->
- ideally, I want software I can own.
- Something fully hackable, debuggable, that I can myself whenever and however I please.
- not really hackable or debuggable
- not really visible / homoiconic
- not standardized in any useful way
- above all, we want software we can just forget about.
<!--- (which components do I need to run this software? vendor sprawl, ..)-->
- if software is so cheap, why can't we make it really good?
- how do we build correct, optimal, integrated software?
- what is the ideal process?
<!--- with exciting new capabilities that are even harder to get right-->
<!--- what does higher order programming look like? what does it even mean?
- what should higher order progrmaming _feel_ like?-->

:::video src="https://www.youtube.com/watch?v=72y2EC5fkcE" title="Tomorrow Corporation Tech Demo" poster="https://i.ytimg.com/vi/72y2EC5fkcE/maxresdefault.jpg"
:::

<!--- beyond performance, the opportunities in a standardized, fully integrated computing stack are very interesting
- now that the cost of writing and rewriting code is nearly zero, what can we do
- how can we build better, correct, integrated software systems
-->

<!--homoiconic software-->
- if we're going to manufacture high quality software with precision, we must first put the "engineering" into "software engineering"
- figure out the system that builds the system, without vague "prompt in a loop" fantasies.
<!--- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale-->
<!--- we could finally put the "engineering" into "software engineering"-->
- 50 years in, there's a thousand ways to do any given thing, and we _still_ haven't solved "works on my machine"
- fortunately, there is now a way out: over the past decade, opinionated formatters have standardized the appearence of code
- now, we should apply the same standardization to the logic of codeb.
<!--- to a lesser degree, pedantic linters have also standardized software somewhat-->
<!--- how far could we go? -->
<!--- what does standardized logic look like? -->
- what are the decisions we can where we can settle on one set of good, coherent options, sort of like how the advent of opinionated formatters put an end to a whole series of unproductive discussions?
- the world is going to run on software, even more so than now, how do we make sure that software is doing what we want?
<!--- (hint: agents are just software, too)-->

# Long Live Code

- Coding is solved. Yet, curiously, bugs are not solved.
<!--- Well: define "coding", define "solved"?-->
- It's a bit of a joke, but if we define coding as transcribing natural language instructions into directionally working symbolic language, then, yes, coding is indeed solved.
<!--- if we increasingly work through other tools, why invest in a language?-->
- What even was the point of code, then? 
<!--- why did we ever write code in the first place?-->
- Should we even think about code at all if it can all be written by AI anyway?
- This feels like asking we care about numbers when calculators can do math. 

- Programming is about problem solving, about iterating to understand a problem and specifying its solution.
- In the olden days of 12 months ago, we used to solve problems with artisinal human-directed next-character-prediction of symbolic code.
- Now, we can usually work at a higher level, a bit like a rich text editor abstracts pencil and parchment.
- And at the next level, instead of one character at a time, I want to understand _everything_: what is the space of all possible software programs to solve my program, and how do I most efficiently get there, where do I go when I'm there, how do I stay in the right place, and so on

[multidimensional software production thingy?](TODO)

- unconstrained software produciton produces a strange sort of software, one that exists purely in linimal space, untethered from and unbothered by reality.
- the tests pass, the screenshots look good, and nothing works.
- somehow, this brand of slop is almost always immediately obvious.
- even with the purple gradients polished away, untethered software never quite solves the right proble, and it's never solved in quite the right way
- smack in the middle of the uncanny valley of software production

- the disconnect of between reality and some  
- to connect software with reality, we need to actually understand and align the problem and its solution with what we want 
- abstraction, yes, but not purely abstraction of specification, also abstraction of exploration, with full specificity available on demand
- otherwise, it's just an art project
<!--- famously, disconnected PMs make the best products?-->
<!--- I've tried that, it doesn't work, I don't want to do that-->
<!--- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.-->

[castle in the sky svg? unvanncy valley maybe?](TODO)

- the production and distribution of code has been so central to software engineering that it's easy to forget that nobody actually cares about code.
- code, beautiful though it may be, is in itself inert and useless.
- mostly, code is just the current way we have of writing down the (alleged) solution to some (perceived) bit-addressable problem.
<!--- worse, nobody really wants _code_ much like nobody wants _software_ or _computers_ in and of themselves (except, perhaps, as space-heaters)-->
- the code is not the product, and usually the software is not the product in and of itself, either.
- as long as there are humans involved at the edges and on the sign off, we need some common ground

- a complex system that works starts from a simple system that works
- in any non-trivial systems I have seen, there is no useful separation of the system and its "specification"
- sometimes the exact sequence of steps matters, sometimes it does not
- the granularity of the specification to care about depends strongly on how standardized the solution is.
<!--- and thus there is no magic abstraction on top of code that will solve _all_ our problems - if the job is solving novel problems.-->
<!---  fine tuning a character controller? better control every bit of entropy. building an email sender? just plop in a framework, we already do this.)-->
<!--- abstracting coding away is in itself a "lossy abstraction"-->
- every attempt to put something _purely_ "above" code and then have it define the behavior of the software with sufficient specificity ends up reinventing code in a worse way (config languages, Gherkin tests, drag and drop coding tools, "APIs will replace everything", etc.)

# Higher Order Programming

- The history of programming is one of monotonically increasing levels of abstraction: from handcrafting gears, to wiring up vacuum tubes, to punching cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script Python.
- that's great, generally.
- climbing the ladder of abstraction yields more output for every bit of input.
- and so we gradually remove ourselves from the cumbersome burden of having to actually spell out _exactly_ what we want the machine to be doing: which electrons? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

[mechanically crafted gear computer thingy](TODO)

<!--- if programming is really just about problem solving, and code is just one medium for formalising solutions, we should expect to see some other evolved forms at varying levels of abstraction-->
- non-code "higher order" programming is not a new idea.
- spreadsheets have existed for decades, game dev people have been doing this for a while, we have now figured out a new way to prompt simple software into existence
- historically, when some more "accessible" programming-adjacent  becomes too common, the "real" programmers no longer consider it programming. thus, excel is not "programming", just like image classification is not "AI"
- and of course, there is the trivial but important point that we're _already_ not writing most code ourselves - the OS, standard libraries, dependency ecosystems, some compiler/transpiler is writing the actual low level code for us, ..

[screenshot of spreadsheet? "the most popular programming language"](TODO)
<!--[the 20 million line problem, casey](TODO)-->

- it follows then that we might want to go all the way, that the ultimate sophistication of programming is not programming at all, but more like "talking to a colleague"? 
- oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple and unconstrained as natural language?
- if we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by formal languages!
- one can only imagine the splendidness of an ecosystem of unconstrained creation!

:::video src="https://www.youtube.com/watch?v=CdWpq2efN8Y" title="Tiny Glade — Release Date Trailer" poster="https://i.ytimg.com/vi/CdWpq2efN8Y/maxresdefault.jpg"
:::

- fortunately, magnificiently, we already have precedent to understand what that low-code, high volume engineering ends up.
- unfortunately, it still involves quite a lot of problem solving and "programming", albeit at a different level.
<!--- the future of software development is lots more like game development-->
<!--- parallels to the early personal software revolution as well-->
- as some gamers may know, early on, game development - even more so than software development - was a complete dredge, and only a self-selected guild of turbonerds could pull it off successfully. 
<!--- everyone has to wire up the pieces themselves, there are few standards, everyone is handrolling OSes and floppy drivers or whatever-->
- then, eventually, the nerds figured out how to package the hard bits into reusable components and evolve more complete, higher level packages of reusable software components we call a "game engine". 

:::video src="https://www.youtube.com/watch?v=WFu1utKAZ18" title="Rayman Legends: The Design Process Within the UbiArt Framework" poster="https://i.ytimg.com/vi/WFu1utKAZ18/maxresdefault.jpg"
:::

- initially, when using an engine, maybe you didn't always get exactly the same level of control, or even hit _quite_ the same highs as the best hardcore engineers. 
- but it was a lot more productive, and it enabled a scale of project
- better tools also enabled more people like game designers, writers, and artists, .. to more directly contribute to the game (visual scripting, material editors, ..)
- so: 
- continuous granularity
- reusable parts, "asset stores" (ShadCN), ..
- lot of the time is just iterating in some more interactive editor
- sometimes you "play" the game, sometimes you edit the game, sometimes you build new tools to help you edit the game, sometimes you watch others play the game, or spawn in some bots to help with a task.
- it's all part of the game

# The System and The Meta System

- we now have agents contributing to software production, and there is a lot of excitement about "software factories" building "agent native" software.
<!--- a myriad of developer-adjacent tools are being rebuilt to become "agent native", while the other half are rebranding into "self-driving factories".-->
- It's not entirely clear what "agent native" means, now where we're "self driving" _to_, but there sure is a lot of momentum in some general direction
- software factory feels like a red herring? like it's inside out? 
- at first glance, agents seem to be an abstraction in _specification_ but I would argue it's a an sbtraction in _production_ of the specification, which points at a different class of product.

- of course, we want good APIs, high performance, open standards ... but that mostly sounds like software the way we should have done anyway.
- more generally, we want software systems that are fully homoiconic and we want systems to udnerstand the systems we're building
- (we're already doing this, but inconsistently and at a small scale..s) 
- unlike in the world of atoms, we get to keep the scaffolding around and just `#IF` it out for deployment!
<!--- "agent native" makes for good marketing and pitch decks, but means little in practice-->
<!--- most things that are good for agents - fast iteration, clean boundaries, programmable software - are good for humans too, we just haven't had the opportunity to the big rewrite until now-->
<!--- like with every technological shift, what are the new abstractions, what is the new shape of software and how do we get there?-->
<!--- and there also just hasn't been a good opportunity to reconsider deepset habits yet -->

[a building under construction with scaffolding](TODO)

- fundamentally, there is not a single test, suite of tests, mathematical proof, or any single definitive gate that you can run to convince me that some non-trivial general purpose program is correct
- doesn't matter whether it's human written or not, software is just very complex
- mathematical proofs are ofc very useful for rigid and fully formalizable systems, but insufficient

- the fundamental problem I have with software is not "oh I wish I had mathematical proof  this program I specified works" it is "how do I know what software to build" and then "how do I know that the specificaiton does what I want"? 
- I don't get any of this feedback from AI 
- there is zero conceptual backpressure from AI
- a "correct proof" or "passing test" means nothing if it's not what I want

- and the only way to assess "correct" for any interesting software is to see it in motion under many different angles, over time
- this is not an intelligence problem!
- it _may_ be possible to "prove" some software to be correct, but I do not know what to prove 
- merely interacting with software 
<!--- correctness = alignment + visibility-->
<!--- correctness is ultimately about alignment, and we can only align on what we can see-->
- I don't know what I want until I see it, and I also don't know what I _don't_ want until I see that too
- correctness is an iterative process, alignment is continuous, the shape is changing
- "correctness" must be specified acrosss many layers to systematically exclude all the things we do _not_ want
<!--- (and ofc there are probabilistic assessments that are even harder to nail down)-->

[lean proof assistent bugs](TODO)

- our tools for building, interacting with, understanding software are pretty primitive
- if everything is code, how do we make sure it's the right code?
- "oh just have the AI tell you if the code is right" but again what is right?
- (in this sense, the "alignment problem" feels much more like a product and legibility problem, and certainly not _merely_ an intelligence problem, which is short term bearish but long term very bullish)
- this does not magically go away with more abstractions or "smarter AI"

- understand the shape of software and the space of all possible software
- make a map
- much better static and dynamic analysis
- "software in motion"
- code is going to run _everything_, even more so than it already does (literally)

[c4 diagram or something?](TODO)

- incremental, iterative, multi-level precision
<!--- who measures the measurer? where is the kernel of truth?-->
<!--- an equivalent problem with proof systems (hello Gödel)-->
- alignment requires visibility, we cannot align what we cannot see
- need a common vocabulary
- correctness is much more iterative, squishy, and multimodal than some cold mathematical proof
- (especially as we get into squishy computation!)
- visibility and precision must be built on a solid foundation, incrementally
- precision requires looking systems..
- .. from many angles, in detail, high low, statically and dynamically, in motion, statically, all sorts of dynamics, ...
<!--- incremental granularity (a la casey muratori)-->
- you can't engineer precision and alignment (i.e. understanding) into a system post-hoc (or at least, only with great difficulty that far exceeds the cost of doing it properly from the start)

# Boring Software

- there is something beautiful about doing the most with the fewest possible parts.
- in programming, a simpler language like C or Go is considered more "elegant" than say C++ or Rust.
- (though very few people would have called either "minimal" at the time they were introduced)
- it's about using the least parts to get the most done, with deliberate "no" to the last 20%.
- this is a great idea, in a multi-language world of heterogeneous stacks.
- "use the best tool for the job"

- unfortunately, _minimal_ is eventually not quite enough, because it _requires_.
- even "multi-paradigm" languages like Java or C# that have organically accumulated more systems-y features over time do not (attempt to) cover the complete spectrum
- general software architecture is no longer exploratory and hasn't been for a while, which is why we have so many frameworks and meta-frameworks solving largely the same problems in largely the same ways
- we want _complete_
<!--- it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases-->
<!--- various languages with different tradeoffs and their own "focus", even multi-paradigm ones-->
<!--- the carcinisation of (managed) languages-->
<!--- however, over time, most serious languages with actual production use evolve an set of common features for building serious software-->
<!--- Go and generics, Java / C# and unsafe / structs / ref, ...
- JVM/CLR by default, Rust on demand-->

[carcinicisation of stacks](TODO)

- boring software must mean _complete_ software, an integrated stack, since that has fewer parts that work better together.
- at first, this smay seem contradictory
- but there is a tremendous advantage to using a single language for *everything*, as shown by the popularity of single language monorepos (suboptimal though they are in various respects)
- much better standardization, much better simulation
- and, if we can manage the perf aspect, much faster and more scalable software too.

[spacex engines](TODO)

- what is the ideal final stack?
if we finally have the unique opportunity to build a completely new programming system, why not just .. throw everything away and start from scratch?
<!--- there are all these suboptimal choices embedded deep into contemporary programming systems
- soo "why not fix all the problems"
- what is the "*ideal* system"
- if "boiling the ocean" suddenly becomes (theoretically) feasible, what sort of landscaping could we do on the software ecosystem-->
- "second system effect"
- what is the pragmatic system we can actually ship soon?
- it is very tempting to get a blank piece of paper and dream up the "perfect system"
- but theoretical 100% optimality probably doesn't really matter as such. 
<!--- pragmatic perfection-->

- I don't want to learn a new and totally different stack, I already know the ones that exist
- I want to use the Web, basically TypeScript, and build with stuff I'm familiar with
- I want to use what I already know, with minimal new learning
- Predictable, known behavior - even if imperfect - is better than something totally new, theoretically perfect thing
- (besides, we usually figure out that the grass isn't quite greener anyway..)

<!--- hardware is getting *more* expensive-->
<!--- we're going to get a lot more software-->
<!--- simulating software is -->

# TypeScript++

- TypeScript is a great foundation
- there is already wide range of prior art in the realm of "TS ergonomics with systems performance", but that is just one aspect of what we'Re trying to do here
<!--- so, before doing something new, the first question is: why not extend what already exists-->
<!--- fully standardized across the stack-->
- static hermes, assembly script, ...
- all in - not incrementally adoptable.
- (though we do have C ABI ofc)

- I like imperative programming
- the current set of modern programming languages is pretty good
- colored functions are fine, Promises are fine actually. microtasks a little weird but whatever
- number is okay as a type actually, it's convenient
- bigint and string as lowercase primitives are fine, not ideal, but fine
- all in all, it's fine, and most importantly: it's familiar
- (... and it's how the web works!)
- no intention of "fixing" anything that is sound but clumsy and can be trivially linted for 
- more importantly, want a complete language that can represent all the things we need, and then constrain by package / library (but it all has to go togetheraaa)
- so let's just get on with it

- what is the minimum edit distance from TypeScript to a universal language that keeps TypeScript's ergonomics and familiarity, is strict and sound and analysable, and also runs reliably at machine speed? 
- basically, what is "TypeScript++"? TS that runs predictably like JVM/CLR/Go with som Rust-y bits
<!--- what is the minimum set of changes / additions we need to good prior art to get what we need-->
- the question is explicitly _not_ what is the "best theortical version if we did TypeScript all over again". 
- instead: "what is the most typescript we can make it, removing only what is absolutely necessary" (default decision = keep)

- what's the most boring thing we could build?
- don't try to be cute or clever or fancy
- no "improvements", only corrections
- don't "fix" what's not _actually_ broken
- only use boring ideas already proven by other languages / libraries / ..
- safe, sound, predictable, and above all: *familiar*

- TypeScript is already tantalizingly close to being a serious, native, _universal_ programming language
<!--- (AssemblyScript and friends fail in the 'feel like TS' department, and Static Hermes does not by design attempt to go "beyond" TS either, which means we need to start from scratch)-->
- the dichotomy between "scripting languages" and "systems languages" no longer makes much sense if it's not humans doing the typing (assuming "compile times" are fast)
<!--- begone with the need for a separate language and stack to run "backend" or "compute heavy" tasks once we "outgrow" node.js or whatever.-->
<!--- .. TypeScript also happens to be the very same language that runs the web, the biggest software platform in the world!-->
<!--- but of course, it would have to really *feel* like TypeScript, not just "look" like TypeScript! as much as possible, TypeScript semantics - far beyond the surface syntax - should be preserved for this to really be a day one language.-->
- TS++ fashions itself as a "superset of a strict subset of TS", which is vaguely reminiscient of the relationship between C and C++
- mechnically, what is the ergonomic ladder of TS++ between TS -> Rust, what are the minimal things to remove for unsoundness, and the minimum features to add to cover the whole universal language spectrum

<!--- well, ideally:
- a language that compiles quickly, runs fast, analyses well, runs everywhere
- above all, a language that is boring, "works as you would expect", so we can innovate in other places
- I don't want to learn new stuff in this area, I want to do better with the stuff I already know
- ideally we want something that can run at the web properly *and* can run systems software at machine speed, and is legible to humans and agents alike.-->
<!--- above all, we need a complete system, a unified method of software production, to reliably produce correct, optimal, integrated software in one standardized way-->
<!--- fully integrated infrastructure, from the bottom to the top of the "stack". -->
<!--- at the centre of it must sit a universal language and runtime.-->

- there are a lot of interesting details in making "TS++" actually work.
- like how we coulds
- you can read all about it at [docs](/docs/language/)

- TS++ is currently very much in alpha
- you can play with it at .. 
- rapidly build out the p90 set needed for most everyday software
