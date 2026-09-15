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

Thus, there is a are rare opportunity to explore and establish entirely new software production processes: probabilistic computing enables new kinds of useful software, which drives demand for new use cases and approaches, _while simultaenously AI drastically reduces the cost of software production and migration. 
In other words, now is the time to reconsider the entire stack.

> To put it quite bluntly: as long as there were no machines, programming was no problem at all; when we had a few weak computers, programming became a mild problem, and now we have gigantic computers, programming has become an equally gigantic problem.
— Edsger Dijkstra, The Humble Programmer 1972

It's a good moment too, as it's now clear that the contemporary software stack is buckling under its own weight; the volume, angle and speed of its new silicon users is just too much.
Alas, the software stack built on the past was, regerettably, not built for this future.
And it's a shame, because the original dream for software very much anticipated this moment.


# The Software We Were Promised

Software was never meant to be like _this_.
The pioneers meant for software to be open, hackable, remixable. 
Hardware has advanced tremendously, fat beyond the capabilities envisiaged half a century ago, and yet, a duller, fragmented version of software dreams has settled in and stayed stuck.
Why?

:::video src="https://www.youtube.com/watch?v=CdWpq2efN8Y" title="Tiny Glade — Release Date Trailer" poster="https://i.ytimg.com/vi/CdWpq2efN8Y/maxresdefault.jpg"
:::

Somewhere deep in the terms of service governing SaaS vendor #47's subprocessor's data stores, the dream of malleable software died.
Just at the time where we _just_ got the tools to - theoretically - 
How do you unify, customise, and evolve the software you need?

Today, if you wanted to own your software suite, you would need access to enough of the sources for each vendor, then make sure they all use the same kinds of stores and interfaces and APIs, and only then could we maybe unify enough of the stack to join "reminders in Notion" with "leads in Salesforce" and "events in calendar".
With the current stack, this is just not a serious option, so we have "connectors".

[xkdc meme with the woblly stack?](TODO)

But connectors are a hack. 
Software itself is a leaky abstraciton.
It is insufficient to patch the problem of incompatible software products with "more and better connectors", because that is solving the symptom; there is a much more fundamental architecture issue, one that it is impossible to correct unless we reconsider the entire stack.

Fortunately, building a new stack just moved from _impossible_ to _very hard_.
To get the most out of software, we need software we can _own_, no rent, no lock-in.
Something fully hackable, debuggable, that is easy to self-host.
Not necessarily because we want to, but because we want to want to

But we're not going to get that software by merely "accelerating" old processes in "self-driving factories". 
To truly industrialise the precise industrialised manufacture of software, we need entirely _new_ processes.
New procesess, not just for studying and developing software behavior, but entirely new lifecycles.

[wind tunnel simulation thingy..? or the wind tunel of the wright brothers? what is the wind tunnel for software?](TODO)

<!--homoiconic software-->
<!--There is an an old joke that computer science really has nothing to do with either computers or science.-->
If software is solved, why is there still so much bad software?
Everybody can vibecode a database, the tests pass, output is "byte-identical", but nobody dares using it.
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

In whatever form, "Programming" is just problem solving: iterating, thinking, working to understand the shape of a problem and then specifying its solution in some repeatable form.
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

At the same time, there is a lot of excitement around "agent native" software and "software factories" - the ideas that we need new more hackable software, and the idea to build them auto semi-autonomously with software that writes software (i.e., agents).
Where, exactly, are we accelerating to?

The temptation to let agents swarm out on a hunch adds a whole new dimension of yak shaving, and yet it's almost never rewarded with anything useful.
Eventually, even my most well intentioned unconstrained production ends in potemkin software where everything _looks_ right but nothing quite works, nothing really fits together, and actually it's not what I wanted at all when I finally _see_ it.

The only way to judge correctness - i.e., does it do what I want - is to see just see the software in motion under many different angles and granularities, inside out.
Notable, this is not an intelligence problem, it's not even an AI problem - I just don't know what I want until I see it, and I also don't know what I _don't_ want until I see that, too.

[a building under construction with scaffolding](TODO)

The "correctness" of a system is an iterative process; its alignment must be continuous, because the shape of the problem and solution must shift in tandem.
This has always been true for any real symbolic software, but it is especially true for probabilstic software.

Under this lense, it's clear that our tools for building, interacting with, understanding software are astoundingly primitive.
We have tools for understanding _code_, testing _code_, analyzing _telemetry_, and the best we can do for "what is the shape of my software?" is .. generating UML diagrams?
I want to understand shape of software and the space of all possible software that solves all the problems I'm interested in, and then navigate that efficiently.
<!--- code is going to run _everything_, even more so than it already does (literally)-->

# Destack

There is something beautiful about doing the most with the fewest parts.
In programming, a more constrained language like C or Go is generally considered more "elegant" than C++ or Rust.
Minimal languages and tools are a great idea in a world of heterogenous stacks with humans doing most of the work.
And in any case, most serious programming languages have a way of accumulating more or less the same set of features over time.

Thus, _minimality_ in isolation is not _quite_ enough, because we do actually need to perform a wide variety of tasks in software, and the less "the system" provides, the more we need to do in diverging ways in userland.
By now, he space of general purpose software architecture is well explored, which is why we have so many frameworks and meta-frameworks solving largely the same problems in largely the same ways

[carcinicisation](TODO)

Indeed, there is a tremendous advantage to using one language for *everything*, as evidenced by the popularity of single language monorepos (suboptimal though they are in many respects).
With a universal language, we would get to use the same domain models, libraries, and build systems across the entire application and lifecycle!

What is the most boring universal stack we can build?
Like, I don't want to learn a new and totally different stack, I already know the modern ones pretty well, and I have better things to do.
The Web is pretty great, TypeScript is pretty great, how far can we take that?. 

[the essence of the bull](TODO)

We have long figured out that opinionated formatters are a great idea to remove silly syntax debates and just standardized the _appearence_ of code once and for all.
Maybe the syntax is not _perfect_ by your personal standards, but it removes unproductive arguments, and that's the benefit.
 <!--and meta-frameworks serves a similar need.-->
Why not just go all the way, and fully standardize the _shape_ of code? Not just syntax, not just linters, but semantics: module and file layouts, type shapes, call trees, services, dependencies, entire architectures. 

<!--- very simple product to manage personal software, personal software stack
- comes with a default stack
- entire familiar web / npm ecosystem at your disposal
- personal package manager
- personal git / jj (including remote) somehow..?

- manage whatever stack you want, but with strong defaults (?)
- best approximation of final Destack we could manage in the existing ecosystem, to get as close as possible and get started immediately-->

# TypeScript++

The univeral final stack will, logically, require the universral final language.
But surprisingly, we do not quite have that universal complete language yet. 
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
Importantly, the question is _not_ what is the "best theortical version if we did TypeScript all over again", but: "what is the minimum edit distance from TypeScript to a universal language that keeps TypeScript's ergonomics and familiarity, is strict and sound and analysable, and also runs reliably at machine speed? 
<!--- basically, what is "TypeScript++"? TS that runs predictably like JVM/CLR/Go with som Rust-y bits-->
<!--- what is the minimum set of changes / additions we need to good prior art to get what we need-->

<!--- TS++ fashions itself as a "superset of a strict subset of TS", which - if you squint - is somewhat reminiscient of the relationship between C and C++.
- mechnically, what is the ergonomic ladder of TS++ between TS -> Rust, what are the minimal things to remove for unsoundness, and the minimum features to add to cover the whole universal language spectrum-->

<!--- there are a lot of interesting details in making "TS++" actually work.
- like how _exactly_ do we combine as much of TS surface feel as possible, while also compiling to a strict sound languaeg? while _also_ enabling up to Rust-level control (and ideally performance)?
- you can read all about it at [docs](/docs/language/)-->
