---
title: "Introducing Destack"
subtitle: "TypeScript++, the last programming language, and the final stack."
date: "2026-09-21"
author: "Florian"
---

Software is entering a Cambrian Explosion, and, it is worth remembering, that means most specimen will go extinct, and the survivors will look very different.
Both products and processes will undergo intense evolutionary pressure, with fast migrations enabling an exhaustive exploration of the hitherto underexplored space of all possible software. 
Eventually, we will arrive at some new final form, resembling the familiar old only in name. 

[illustration of cambrian explosion carnage](TODO)

If you need software, there are two options:
- **Build** and take on the uncertain process of artisinal production directly, including the risk for supply chain attacks, vendor sprawl, and deployment hell, only to most likely get half baked software and an additional maintenance burden.
- **Buy** and pay the cost and integration required to get up and running, accept somebody else's slightly suptimal process as a critical dependency, and then be stuck paying rent to some vendor for at least the next good while.

But now, suddenly, we have a rare dual opportunity: probabilistic computing enables new kinds of useful software, which drives new use cases and exploration, _and_ simultaenously AI drastically reduces the cost of software production and migration. 
It's the perfect time to reconsider everything.

The contemporary software stack is beginning to buckle under its own weight, and the volume and angle of its new silicon users is too much.
Alas, the software stack built on the past was, regerettably, not built for this future.
And it's a damn shame, because the original dream for software very much waited for this moment.

# The Software We Were Promised

Software wasn't meant to be like _this_.
Software was meant to be open, hackable, remixable. 
Hardware has advanced tremendously, incredible, and yet, a duller, fragmented version of software dreams has settled in and stayed stuck.  
What gives?

:::video src="https://www.youtube.com/watch?v=CdWpq2efN8Y" title="Tiny Glade — Release Date Trailer" poster="https://i.ytimg.com/vi/CdWpq2efN8Y/maxresdefault.jpg"
:::

Somewhere, deep in the terms of service of 47 different SaaS vendors with disjoint data stores, the software dream died with a whimper.
It is insufficient to patch this problem with "connectors", the problem is a much more fundamental architecture issue, one that it is impossible to correct unless we reconsider the entire stack.

Building a new stack is just on the cusp of moving from _impossible_ to _very hard_.
To get the most out of software, we need software we can _own_, no rent, no lock-in.
Something fully hackable, debuggable, that is easy to self-host.
Not necessarily because we want to, but because we want to want to

And we're not going to get that software by merely "accelerating" old processes in "self-driving factories". 
To truly industrialise the precise industrialised manufacture of software, we need entirely _new_ processes.
New procesess, not just for studying and developing software behavior, but entirely new lifecycles.

[wind tunnel simulation thingy..? or the wind tunel of the wright brothers? what is the wind tunnel for software?](TODO)

<!--homoiconic software-->
<!--There is an an old joke that computer science really has nothing to do with either computers or science.-->
If software is solved, why is there still so much bad software?
<!--Why do we only have like three databases that everyone trusts, instead of either just one, or infinitely many?-->
Everybody can vibecode a database, the tests pass, but nobody dares using it.
Strange.
Something is clearly amiss.
How do we put the "engineering" into "software engineering"?

The production of software has many issues, mostly rooted in the inscrutable complexity of the monster we've made: it takes >50M LoC to get anything on screen, too many fragmented tools and services, too hard to simulate accurately (what are bugs, after all).
Our inability to accurately simulate software behavior is ironic considering our medium is purely digital, and thus, if the stack actually worked like it should, there is no excuse for software being _less_ than perfectly reliably and maximally fast, seven nines, picture perfect. 

# Higher Order Programming

The history of programming is one of increasing levels of abstraction: from handcrafting gears, to wiring up vacuum tubes, to punching cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script whatever.
And that's great.

Climbing the ladder of abstraction yields more output for every bit of input.
We gradually remove ourselves from the cumbersome burden of having to actually spell out _exactly_ what we want the machine to be doing: which electrons? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

[mechanically crafted gear computer thingy](TODO)

Historically, whenever some more accessible form of programming becomes too common, the "real" programmers no longer consider it programming.
Thus, Excel is not "programming", just like image classification is no longer "AI" - and soon, presumably, voice recognition, chatbots, and agentiveness will blend into boring software like the magic of the internet did.

In whatever form, "programming" is just problem solving: iterating, thinking, working to understand the shape of a problem and then specifying its solution in some repeatable form.
We used to solve software-shaped problems with artisinal human-directed next-character-prediction of symbolic code, but really, it doesn't even have to be any formal language at all - recipe writing is programming, too.
<!--All that really matters is repeatable specification, in any useful form.-->

<!--The production and distribution of code has been so central to software engineering that it's easy to forget that nobody actually cares about the code.-->
<!--- Code, beautiful though it may be, is in itself inert and useless, just like the software it describes.-->
<!--- if we increasingly work through other tools, why invest in a language?-->
<!--- So, now that we don't _have_ to, should we even think about code at all?-->
<!--- That seems like the wrong question, akin to:-->
<!--- Why care about numbers when calculators exist?-->

Unlike the jump from machine code to Python, the jump from hand-directing edits to agentic coding is much more about the _abstraction of production_ than _abstraction of specification_.
It feels a lot like the invention of the rich text editor with spellchecks abstracts over handwriting with pencil on parchment.
Editing becomes easier, the specification remains.

<!--The observant programmer may now note the obvious reality that we're _already_ not writing most code ourselves, even when we still used to hand-write code, because of the massive stack separating bits from atoms - all those millions upon millions of lines between the "raw" code and the actual hardware. 
But we don't talk about that, and code _is_ programming.-->

<!--Yet, the idea of programming beyond code almost as old as code itself.-->
Right from the very start, the computing pioneers dreamed of interacting with the computer with natural language and multimodal inputs.
Spreadsheets kicked off the first personal computer revolution in earnest, thereby replacing entire rooms of people performing menial tabulations by hand with a much more approachable and programmable interface.
(And thereby also giving those people higher level jobs.)

[that old IBM ad about replacing 50 engineers or something?](TODO)

To this day, the humble spreadsheet is _still_ the most popular application platform and surprisingly hard to displace.
But, of course, we generaly do not buy professional software that comprises purely a spreadsheet, in much the same way that we don't buy software purely made of connectors
There is a little more to "real" higher level software.

The history of game development parallels and often previsages the history of software development, simply because games face even tighter constraints on everything, and even more competitive pressure to get the most out of hardware, all the while working with multidiscplinary teams. 
Early on, game development was also a complete schlep, and only a tiny guild of brilliant nerds could pull off presentable commercial games.

To get a game started, everyone had to write their own graphics, networking, scripting, asset pipelines, editors - a whole engine for every game, on top of the actual game! 
Then, eventually, we figured out how to package the hard bits into reusable components and evolve more complete, higher level packages of reusable software components we call a "game engine". 

[the first version of unreal engine or something?](TODO)
<!--:::video src="https://www.youtube.com/watch?v=WFu1utKAZ18" title="Rayman Legends: The Design Process Within the UbiArt Framework" poster="https://i.ytimg.com/vi/WFu1utKAZ18/maxresdefault.jpg"
:::-->

Initially, when using an engine, maybe you didn't always get exactly the same level of control, or even hit _quite_ the same high notes as the best hardcore engineers could without. 
But it was a lot more productive, and it enabled a scale of project and a type of contributor that was impossible before.
Non-programmer people like designers, writers, and artists, could now contribute _directly_ to the product, be it via Lua, visual scripting, material editors, or more advanced in-game level designers.

The history of game development never "removed" code, even as a lot of it was abstracted for many development use cases that previously required it, and you can now get very far (and sometimes even to the end!) into building a real game without never looking at any "real" code at all.
Now, when building a game, sometimes you "play" the game, sometimes you edit the game, sometimes you build new tools to help you edit the game, sometimes you watch others play the game, and so it goes.

# The System and The Meta System

Fundamentally, there is not a test, suite of tests, certificate, proof, or _any_ single "definitive" gate that you can run to convince me that some non-trivial software program is correct.
Complex software systems span many granuliarities, and human written or not, misalignment can hide in any of them.
Mathematical proo, passing tests, and green gates all mean nothing if it's not what I actually wanted.

There is a lot of excitement around "agent native" software and "software factories" - the ideas that we need new more hackable software, and the idea to build them auto semi-autonomously with software that writes software (i.e., agents).
What, exactly, are we accelerating?

The temptation to let agents swarm out on a hunch adds a whole new dimension of yak shaving, and yet it's almost never rewarded with anything useful.
Eventually, even my most well intentioned unconstrained production ends in potemkin soup where everything _looks_ right but nothing quite works, nothing really fits together, and actually it's not what I wanted at all when I finally _see_ it.

The lack of visibility and continous alignment causes a weird sort of drift from reality, which ultimately still makes it surprisingly tricky to automate production outside of routine migrations and well scoped autofixes.
When everything is a great idea, everything is implemented immediately, the software just has no shape at all; and worse, there is very little "conceptual backpressure" from reality, so we can't even do much better next time.

- the only way to judge correctness - i.e., does it do what I want - for any interesting software is to see it in motion under many different angles, over time
- this is not an intelligence problem!
- it _may_ be possible to "prove" some software to be correct, but I do not know what to prove 
- merely interacting with software 
<!--- correctness = alignment + visibility-->
<!--- correctness is ultimately about alignment, and we can only align on what we can see-->
- I don't know what I want until I see it, and I also don't know what I _don't_ want until I see that too

- correctness is an iterative process, alignment is continuous, the shape is changing
- this is true for symbolic software, but it is especially true for probabilstic software.
- "correctness" must be specified acrosss many layers to systematically exclude all the things we do _not_ want
<!--- (and ofc there are probabilistic assessments that are even harder to nail down)-->
- if everything is code, how do we make sure it's the right code?

[a building under construction with scaffolding](TODO)

Our tools for building, interacting with, understanding software are astoundingly primitive.
- "oh just have the AI tell you if the code is right" but again what is right?
- (in this sense, the "alignment problem" feels much more like a product and legibility problem, and certainly not _merely_ an intelligence problem, which is short term bearish but long term very bullish)
- this does not magically go away with more abstractions or "smarter AI"

- understand the shape of software and the space of all possible software
- make a map
- much better static and dynamic analysis
- "software in motion"
- code is going to run _everything_, even more so than it already does (literally)

[blind man and the elephant vs C4 diagram and xray lol?](TODO)

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

# Destack

- there is something beautiful about doing the most with the fewest parts.
- in programming, a simpler language like C or Go is considered more "elegant" than C++ or Rust.
- (now, few people would have called either "minimal" at the time they were introduced)
<!--- it's about using the least parts to get the most done, with deliberate "no" to the last 20%.-->
- minimal languages is a great idea, in a multi-language world of heterogeneous stacks.
<!--- "use the best tool for the job"-->

- unfortunately, _minimality_ in isolation is not _quite_ enough, because we do actually need to perform a wide variety of tasks, and the less "the system" provides, the more we need to do in diverging ways in userland.
- even "multi-paradigm" languages like Java or C# that have organically accumulated more systems-y features over time do not (attempt to) cover the complete spectrum
- general software architecture is no longer exploratory and hasn't been for a while, which is why we have so many frameworks and meta-frameworks solving largely the same problems in largely the same ways
- we want _complete_
<!--- it's genuinely pleasing to get so much out of relatively little syntax that covers so many use cases-->
<!--- various languages with different tradeoffs and their own "focus", even multi-paradigm ones-->
<!--- the carcinisation of (managed) languages-->
<!--- however, over time, most serious languages with actual production use evolve an set of common features for building serious software-->
<!--- Go and generics, Java / C# and unsafe / structs / ref, ...
- JVM/CLR by default, Rust on demand-->

[carcinicisation](TODO)

- there is a tremendous advantage to using a single language for *everything*, as shown by the popularity of single language monorepos (suboptimal though they are in various respects)
- much better standardization, much better simulation
- and, if we can manage the perf aspect, much faster and more scalable software too.

- I don't want to learn a new and totally different stack, I already know the ones that exist
- I want to use the Web, basically TypeScript, and build with stuff I'm familiar with
- I want to use what I already know, with minimal new learning

- in essence, what's the most boring thing we could build, that still does everything we need?
- don't try to be cute or clever or fancy
- no "improvements", only corrections
- don't "fix" what's not _actually_ broken
- only use "boring" ideas already proven by other languages / libraries / standards / ..
- safe, sound, predictable, and above all: *familiar*

[the essence of the bull](TODO)

<!--- Predictable, known behavior - even if imperfect - is better than something totally new, theoretically perfect thing-->

- Oinionated formatters have already standardized the _appearence_ of code.
- Why not also standardize the _shape_ of code? Not just syntax, not just linters, but semantics: state trees, call flows, dependencies, entire architectures. 
<!--- Standardization at all granularities.-->

- As always, the first instinct is to search for something that already exists - Python? TypeScript?
- We can get very far with just TypeScript, Web APIs

- very simple product to manage personal software, personal software stack
- comes with a default stack
- entire familiar web / npm ecosystem at your disposal
- personal package manager
- personal git / jj (including remote) somehow..?

- manage whatever stack you want, but with strong defaults (?)
- best approximation of final Destack we could manage in the existing ecosystem, to get as close as possible and get started immediately

# TypeScript++

- The complete final stack will, eventually, require the complete final language.
- But surprisingly, we do not quite have that universal complete language yet. 
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
- like how _exactly_ do we combine as much of TS surface feel as possible, while also compiling to a strict sound languaeg? while _also_ enabling up to Rust-level control (and ideally performance)?
- you can read all about it at [docs](/docs/language/)

# Hello

<!--- Destack TS++ is currently very much in alpha
- you can play with it at .. 
- rapidly build out the p90 set needed for most everyday software-->
