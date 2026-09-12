---
title: "Introducing Destack"
subtitle: "Evolving TypeScript into the Last Programming Language"
date: "2026-09-21"
author: "Florian"
---

Software is entering a Cambrian Explosion, and, it is worth remembering, that means most specimen will go extinct, and the survivors will look very different.
Both products and processes will undergo intense evolutionary pressure, with instant migrations enabling a ruthless exploration of the full space of possible software. 
Until, eventually, we arrive at some new final form, resembling the familiar old only in name. 
<!--- (and, perhaps, sidebar placement).-->

[illustration of cambrian explosion carnage](TODO)

- The _original_ promise of software has, so far not been realized, and a duller version of it has settled in by sheer inertia.  
- Somewhere, in the midst of 20 different SaaS vendors with 2 second loading pages and incompatible formats, the beauty of orchestrating logic on top with enlightened silicon was lost.
- not in the "install a connector" sense, but in the drag a thing from one tab into the other sense, in the "ask the computer to modify the software just for you" sense.

- If you need software, there are two sad choices: build or buy.
- Choose build, and endure the uncertain process of artisinal production through dependency forest, supply chain attacks, vendor sprawl, and deployment hell, only to get some clunky software and an additional maintenance burden.
- Choose buy, and endure the integration required to get up and running, accept somebody else's slightly suptimal process as a critical dependency, and then be stuck with that vendor for at least the next few years. 

<!--- very hard to run software locally-->
- The promised land of _great_ software will not arrive by vaguely gesturing at the computer to spraypaint a blend of past software, nor by blindly "accelerating" old processes in "self-driving factories". 
<!--- (if for no other reason than that competitive pressures will demand it)-->
- No. If we want _new_ software, beautiful software, fast software, _great_ software, we also need _new_ processes.
- Now that we finally have the chance: what _should_ software be?

- so, ideally, I want software I can own and hack. no rent, no lock-in.
- Something fully hackable, debuggable, that I can myself whenever and however I please.
- not really hackable or debuggable
- not really visible / homoiconic
- not standardized in any useful way
- above all, we want software we can just forget about.
<!--- (which components do I need to run this software? vendor sprawl, ..)-->

[that old IBM ad about replacing 50 engineers or something?](TODO)

<!--homoiconic software-->
- The greatness within software, we need to industrialise software production.
- artisinal production will not cut it. we must first put the "engineering" into "software engineering".
- scaling the high quality manufacture of any product requires first understanding, with considerable precision, the dynamics of the process
- aerodynamics for software engineering, if you will 
- our medium is purely digital and - if done right - fully testable, there is no excuse for software being _less_ than perfectly reliably and maximally fast, seven nines included. 
<!--- figure out the system that builds the system, without vague "prompt in a loop" fantasies.-->
<!--- The _raison d'être_ of Destack is to enable the precise manufacture of high quality software at scale-->
<!--- we could finally put the "engineering" into "software engineering"-->
<!--- 50 years in, there's a thousand ways to do any given thing, and we _still_ haven't solved "works on my machine"-->

[wind tunnel simulation thingy..? or the wind tunel of the wright brothers? what is the wind tunnel for software?](TODO)

# Higher Order Programming

- The history of programming is one of increasing levels of abstraction: from handcrafting gears, to wiring up vacuum tubes, to punching cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script whatever.
- And that's great.
- Climbing the ladder of abstraction yields more output for every bit of input.
- We gradually remove ourselves from the cumbersome burden of having to actually spell out _exactly_ what we want the machine to be doing: which electrons? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

[mechanically crafted gear computer thingy](TODO)

<!--- if programming is really just about problem solving, and code is just one medium for formalising solutions, we should expect to see some other evolved forms at varying levels of abstraction-->
- The idea of extending programming beyond code is perhaps older than code itself.
<!--- And non-code "higher order" programming is not a new idea.-->
- Spreadsheets sparked the first personal computer revolution, TODO, and game developers have been doing working with non-code abstractions for decades now. 
- we have now figured out a new way to prompt simple software into existence

- Historically, when some more "accessible" programming-adjacent  becomes too common, the "real" programmers no longer consider it programming. thus, excel is not "programming", just like image classification is not "AI"
- And of course, there is the trivial but important point that we're _already_ not writing most code ourselves - the OS, standard libraries, dependency ecosystems, some compiler/transpiler is writing the actual low level code for us, ..

[screenshot of spreadsheet? "the most popular programming language"](TODO)
<!--[the 20 million line problem, casey](TODO)-->

- It follows then that we might want to go all the way, that the ultimate sophistication of programming is not programming at all, but pure unconstrained natural language, more like "talking to a colleague"? 
- Oh, how great software could be, how magnificient, how accessible, if only we could make programming as simple and unconstrained as natural language?
- If we didn't have to write code at all, nor trouble ourselves with any of the nuances and rigor imposed by formal languages!
- One can only imagine the splendidness of an ecosystem of unconstrained creation!

:::video src="https://www.youtube.com/watch?v=CdWpq2efN8Y" title="Tiny Glade — Release Date Trailer" poster="https://i.ytimg.com/vi/CdWpq2efN8Y/maxresdefault.jpg"
:::

- Fortunately, magnificiently, we already have precedent to understand where low-code, high volume engineering might end up.
- Unfortunately, it still involves quite a lot of problem solving and "programming", albeit at a different level.

<!--- the future of software development is lots more like game development-->
<!--- parallels to the early personal software revolution as well-->
- Early on, game development - even more so than software development - was a complete schlep, and only a self-selected guild of obsessed nerds could pull it off.
- Everyone had to write their own graphics, networking, scripting, asset pipelines, editors - a whole engine for every game, on top of the actual game! 
- Then, eventually, the nerds figured out how to package the hard bits into reusable components and evolve more complete, higher level packages of reusable software components we call a "game engine". 

:::video src="https://www.youtube.com/watch?v=WFu1utKAZ18" title="Rayman Legends: The Design Process Within the UbiArt Framework" poster="https://i.ytimg.com/vi/WFu1utKAZ18/maxresdefault.jpg"
:::

- Initially, when using an engine, maybe you didn't always get exactly the same level of control, or even hit _quite_ the same high notes as the best hardcore engineers could without. 
- But it was a lot more productive, and it enabled a scale of project and a type of contributor that was impossible before.
- People like game designers, writers, and artists, could finally contribute directly to the game, be it via Lua, visual scripting, material editors, or customised in-game level designers.

- so: 
- continuous granularity
- reusable parts, "asset stores" (ShadCN), ..
- lot of the time is just iterating in some more interactive editor
- sometimes you "play" the game, sometimes you edit the game, sometimes you build new tools to help you edit the game, sometimes you watch others play the game, or spawn in some bots to help with a task.
- it's all part of the game

# Long Live Code

- Programming is problem solving: iterating, thinking, working to understand a problem and specifying its solution in some repeatable form.
- It doesn't even have to be computer code or any formal language at all - recipe writing is programming, too.
- In the olden days of 12 months ago, we used to solve software-shaped problems with artisinal human-directed next-character-prediction of symbolic code.

- The production and distribution of code has been so central to software engineering that it's easy to forget that nobody actually cares about code.
- Code, beautiful though it may be, is in itself inert and useless, just like the software it describes.
<!--- if we increasingly work through other tools, why invest in a language?-->
- So, now that we don't _have_ to, should we even think about code at all?
- Asking whether code is still meaningful because AI exists feels like asking why care about numbers when calculators exist. 

- Unlike the jump from machine code to assembly, and then assembly to C, the jump from hand-directing edits to having an agent do it is much more about _abstraction of process_ than _abstraction of specification_.
- Basically, agentic coding feels a lot like the invention of the rich text editor with spellchecks abstracts over handwriting with pencil on parchment.
- Editing becomes easier, substantiation is simpler, figuring out what to specify is stil lhard.
- And if the _specification_ drifts from reality, we get bizzaro slop. 

- In that sense, higher level direction is not an abstraction at all, it is an enabler for a new kind of process.
- The ideal level of code for programming has nothing to do with either code or programming.
- Instead of one character at a time, one program at a time, I want to understand _everything_ at all granularities: what is the space of all possible software programs to solve the space of problems I have, and how do I most efficiently get there, where do I go once I'm there, how do I stay in the right place as the problem shifts?

:::video src="https://www.youtube.com/watch?v=72y2EC5fkcE" title="Tomorrow Corporation Tech Demo" poster="https://i.ytimg.com/vi/72y2EC5fkcE/maxresdefault.jpg"
:::

- Unconstrained software produciton produces a strange sort of software, one that exists purely in linimal space, untethered from and unbothered by reality.
- The tests pass, the screenshots look good, and nothing works.
- Left to its own devices, the software factory lands _perfectly_ in the middle of the uncanny valley of software.
<!--- somehow, this brand of slop is almost always immediately obvious.-->
- even with the purple gradients polished away, untethered software never quite solves the right proble, and it's never solved in quite the right way

- the disconnect of between reality and some  
- to connect software with reality, we need to actually understand and align the problem and its solution with what we want 
- abstraction, yes, but not purely abstraction of specification, also abstraction of exploration, with full specificity available on demand
- otherwise, it's just an art project
<!--- famously, disconnected PMs make the best products?-->
<!--- I've tried that, it doesn't work, I don't want to do that-->
<!--- I want to be _more_ in the details than ever, I want the code to be right and look right, I want to understand every byte, every cycle, every pixel.-->

[simpsons meme about serving undifferentiated slop (food)](TODO)

- a complex system that works starts from a simple system that works
- in any non-trivial systems I have seen, there is no useful separation of the system and its "specification"
- sometimes the exact sequence of steps matters, sometimes it does not
- the granularity of the specification to care about depends strongly on how standardized and thus how structured the solution space is.
- there is not, and cannot be, some magic abstraction on top of code that will solve _all_ our problems - if the job is solving novel problems.

- There is a more promising angle and prerequisite to increased production capacity: standardization.
- Over the past decade, opinionated formatters have standardized the _appearence_ of code.
- Can we also standardize the _shape_ of code? Not just syntax, but semantics: state trees, call flows, dependencies, entire architecture. 
- Standardization at all granularities.

- what are the decisions we can where we can settle on one set of good, coherent options, sort of like how the advent of opinionated formatters put an end to a whole series of unproductive discussions?
- the world is going to run on software, even more so than now, how do we make sure that software is doing what we want?

# The System and The Meta System

- We now have agents contributing to software production, and there is a lot of excitement about "agent native" software.
- Software to make software, it all lines up very nicely, and it makes for great marketing.
- Agents are a new kind of software, for sure, but fundamentally it's all still software.
- The more pertinent question then is: how do we build complex software in the first place? What does the ideal supporting system look like?

- Of course, we want good abstractions, high performance, open standards.
- Essentially, all the things that software should have done anyway, but now we get to take them to the limit: complete, standardized software systems that are fully _homoiconic_ and hackable at every level of granularity.
- Beyond aiding in construction and maintenance, the "software to build the software", the "meta system", is instrumental in figuring out what we should even be specifying in the first place.

- Fundamentally, there is not a single test, suite of tests, mathematical proof, or any single definitive gate that you can run to convince me that some non-trivial general purpose program is correct.
- doesn't matter whether it's human written or not, software is just very complex
- mathematical proofs are ofc very useful for rigid and fully formalizable systems, but insufficient

- the fundamental problem I have with software is not "oh I wish I had mathematical proof  this program I specified works" it is "how do I know what software to build" and then "how do I know that the specificaiton does what I want"? 
- I don't get any of this feedback from AI 
- there is zero conceptual backpressure from AI
- a "correct proof" or "passing test" means nothing if it's not what I want
- I'm not ready to vouch for a 27k line "proof" I don't understand verifying some properties I also don't understand

[lean proof assistent bugs](TODO)

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

[a building under construction with scaffolding](TODO)

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

[c4 diagram or Uml diagram or something?](TODO)

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
- (now, few people would have called either "minimal" at the time they were introduced)
- it's about using the least parts to get the most done, with deliberate "no" to the last 20%.
- this is a great idea, in a multi-language world of heterogeneous stacks.
- "use the best tool for the job"

- unfortunately, _minimality_ in isolation is not quite enough, because it _requires_.
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

[the essence of the bull](TODO)

- I don't want to learn a new and totally different stack, I already know the ones that exist
- I want to use the Web, basically TypeScript, and build with stuff I'm familiar with
- I want to use what I already know, with minimal new learning
- Predictable, known behavior - even if imperfect - is better than something totally new, theoretically perfect thing
- (besides, we usually figure out that the grass isn't quite greener anyway..)

<!--- hardware is getting *more* expensive-->
<!--- we're going to get a lot more software-->
<!--- simulating software is -->

- what's the most boring thing we could build?
- don't try to be cute or clever or fancy
- no "improvements", only corrections
- don't "fix" what's not _actually_ broken
- only use boring ideas already proven by other languages / libraries / ..
- safe, sound, predictable, and above all: *familiar*

# TypeScript++

- The complete final stack logically requires the complete final language
- And somewhat surprisingly, we do not quite have that universal complete language yet. 
- Of course, there are languages you could _contort_ to target all platforms and write everything from systems software to API services to web apps.
- Besides, if we want the deep sort of analysis and standardization we seek, it is much easier to start from scratch than try to retrofit this into an existing ecosystem.

- The closest thing we have to a universal language is TypeScript.
- TypeScript is actually pretty great.
- Everyone knows TypeScript. 
- Critically, the web runs on TypeScript (JavaScript).
- the dichotomy between "scripting languages" and "systems languages" no longer makes much sense if it's not humans doing the typing (assuming "compile times" are fast)

- TypeScript is already tantalizingly close to being a serious, native, _universal_ programming language
- there is already wide range of prior art in the realm of "TS ergonomics with systems performance" we can learn from, but that is just one aspect of what we'Re trying to do here
<!--- so, before doing something new, the first question is: why not extend what already exists-->
<!--- fully standardized across the stack-->
- static hermes, assembly script, ...
- all in - not incrementally adoptable.
- (though we do have C ABI ofc)

- what we want, then, is effectively "TypeScript++".
- all the soundness warts removed, just enough features added to enable memory safe systems programming, and a familiar enough runtime to require no new learning (even if it's just for code review).
- what is the minimum edit distance from TypeScript to a universal language that keeps TypeScript's ergonomics and familiarity, is strict and sound and analysable, and also runs reliably at machine speed? 
- basically, what is "TypeScript++"? TS that runs predictably like JVM/CLR/Go with som Rust-y bits
<!--- what is the minimum set of changes / additions we need to good prior art to get what we need-->
- the question is explicitly _not_ what is the "best theortical version if we did TypeScript all over again". 
- instead: "what is the most typescript we can make it, removing only what is absolutely necessary" (default decision = keep)

- TS++ fashions itself as a "superset of a strict subset of TS", which - if you squint - is somewhat reminiscient of the relationship between C and C++.
- mechnically, what is the ergonomic ladder of TS++ between TS -> Rust, what are the minimal things to remove for unsoundness, and the minimum features to add to cover the whole universal language spectrum

- there are a lot of interesting details in making "TS++" actually work.
- like how we coulds
- you can read all about it at [docs](/docs/language/)

- TS++ is currently very much in alpha
- you can play with it at .. 
- rapidly build out the p90 set needed for most everyday software
