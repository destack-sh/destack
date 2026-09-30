---
title: "Introducing Destack"
subtitle: "The final stack for personal software (for real this time)."
date: "2026-10-05"
author: "Florian"
---

Software is entering another Cambrian Explosion, and, it is worth remembering, that means most specimen will go extinct, and the survivors will look very different.
Both products and processes will evolve under intense competition, with free migrations enabling the full exploration of the space of all possible software, until a new optima is found.

<!-- Publication license pending: Christian Jégou / Science Source. -->
:::figure width="600" src="./cambrian-sea.jpg" alt="Illustration of Cambrian marine life, with Opabinia swimming above trilobites, spiny animals, and sponges."
[Cambrian marine life](https://es.knowablemagazine.org/content/articulo/alimentos-ambiente/2026/como-los-herbivoros-obtienen-aminoacidos-esenciales) — Christian Jégou, Science Source.
:::

It's good timing too, because the current stack is fundamentally broken:
we've got a thousand little software silos, each with their own slightly incompatible slice of the stack, wrapping its own control and data and auth and compute and telemetry planes, awkwardly encapsulated from its users, and all but impossible to modify or even properly integrate.

<!--Now, we have a rare opportunity to explore new software production processes as probabilistic computing enables new kinds of useful software, which drives demand for new use cases and approaches, while, simultaneously, AI drastically reduces the cost of software production and migration.-->
<!--It is the time to reconsider the entire stack.-->

> To put it quite bluntly: as long as there were no machines, programming was no problem at all; when we had a few weak computers, programming became a mild problem, and now we have gigantic computers, programming has become an equally gigantic problem.
>
> — Edsger Dijkstra, *The Humble Programmer* (1972)

Half a century later since the last software crisis, the software stack is once again buckling under the weight, volume and speed of a more powerful machine.
Once more, we will have to retrace from the beginning, reconsider what programming even means, and reshape what software should look like for the next century.

# The Software That Could Be

Software was never meant to be like _this_.
The pioneers meant for software to be open, hackable, remixable - and fast.
Hardware has advanced far beyond the capabilities envisioned all those decades ago, and yet, a duller, infinitely fragmented version of software has settled in and stayed stuck by sheer inertia.

Of course, "malleable software" and "end-user programming" have been valiantly again and again - only to die again and again.
Sort of.
The closest to malleable software we have today are constrained sandboxes like Excel and Notion, which are useful, but not _general_, and so we are stuck with a thousand little rented software silos.

:::figure width="600" src="./tiny-glade.gif" alt="In Tiny Glade, drawing a path through a wall creates an archway automatically."
[Path editing in Tiny Glade](https://www.youtube.com/watch?v=CdWpq2efN8Y) — Pounce Light, trailer excerpt.
:::

Yet, our new alien machine users urgently demand the broadest possilbe access to the entire software stack, with as many degrees of freedom as we can safely provide, up and down and left and right. 
Agents are on pace to outnumber human users by an order of magnitude or two very soon, but they are awkwardly caged in by last century's software stack.

The key issue with "malleable software" has been that approximately nobody _wants_ to build their own software.
Building custom software was enticing but difficult, it never ends, rarely works _quite_ as well, and so there was little reason to even attempt to own the stack.

Now, the equation has flipped, and it is almost trivially easy to build your own software, but nothing quite fits together yet.
Just like it's hard to build custom software that works well across all systems, it is difficult to deploy agents if software is fragmented across thousands of little silos you do not own or control.

For example, how do you join "reminders in Notion" with "leads in Salesforce" and "events in Outlook"?
With real, transactional guarantees, in real time, at scale?
It simply does not compute - the depth of integration we need is impossible with the current approach.

:::figure width="640" src="./wright-flyer-wind-tunnel.jpg" alt="A full-size Wright Flyer replica mounted on a test stand inside the Ames wind tunnel, with two engineers standing beside it."
[Wright Flyer replica in the Ames wind tunnel](https://www.nasa.gov/image-article/wright-flyer/) — NASA, public domain.
:::

The predicament of the current software stack goes far beyond a mere lack of integration:
it's not just that things don't work well _together_, it's that they don't work well _at all_.
If software is solved, why is there still so much bad software?
How do we put the "engineering" into "software engineering"?

The brute "acceleration" of old processes with "self-driving factories" will not magically yield a better stack with better software, just more of the same stack.
To finally industrialise the precise manufacture of quality software, we need entirely _new_ processes, not just for building software, but for understanding and studying software from all angles.

# Higher Order Programming

The history of programming is one of increasing levels of abstraction: from handcrafting gears, to wiring up vacuum tubes, to punching cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script however it likes.
And that's great.

Climbing the ladder of abstraction yields more output for every bit of input.
We gradually remove ourselves from the cumbersome burden of having to actually spell out _exactly_ what we want the machine to be doing: which electrons? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

:::figure width="440" src="./babbage-engine.jpg" alt="The 1832 demonstration portion of Babbage’s Difference Engine No. 1, with its columns of brass gears and hand crank."
[Difference Engine No. 1, 1832](https://commons.wikimedia.org/wiki/File:Babbages_difference_engine_1832.jpg) — Sebastian Wallroth, public domain.
:::

"Programming" is just problem solving: iterating, thinking, working to understand the shape of a problem and then specifying its solution in some repeatable form.
We used to solve software-shaped problems with artisinal human-directed next-character-prediction of symbolic code, but really, it doesn't even have to be any formal language at all - recipe writing is programming, too.

The idea of programming beyond code is almost as old as code itself, and there are many existing approaches to higher order programming.
Indeed, game developers have been doing this for decades!
Early on, game development was also a complete schlep, and only a tiny guild of brilliant nerds could pull off presentable commercial games.

In the early days, to get a game started, everyone had to write their own graphics, networking, scripting, asset pipelines, editors - a whole engine for every game, on top of the actual game!
Then, eventually, we figured out how to package the hard bits into reusable components and evolve more complete, higher level packages of reusable software components we call a "game engine".

:::figure width="480" src="./ibm-extra-engineers.jpg" alt="IBM’s 1951 advertisement, 150 Extra Engineers, showing rows of engineers doing calculations."
[150 Extra Engineers, 1951](https://commons.wikimedia.org/wiki/File:IBM_150_Extra_Engineers_1951.jpg) — IBM.
:::
<!--:::figure width="640" src="./ubiart.gif" alt="An artist assembles and poses a hand-drawn character directly in UbiArt, with the artwork and animation rig side by side."
[Character editing in UbiArt](https://www.youtube.com/watch?v=B_QhZYTukac&t=35s) — Ubisoft, demonstration excerpt.
:::-->

Initially, developers using game engines didn't get quite the same level of control, or hit _quite_ the same high notes as those without.
But it was a lot more productive, and it enabled a scale of project and a type of contributor that was impossible before - designers, writers, and artists, could now contribute _directly_, be it via Lua, visual scripting, material editors, or more advanced in-game level designers.

Games never superseded code, even as a lot of code was abstracted away for use cases that previously required it.
It's still important, but, thanks to modern game engines, you can now build commercial games without thinking in code.

The analogy of the game engine also applies to the iterative deployment of software, especially as we bifurcate into stable "platform software" building blocks that serve "userland software". 
When building a game (or piece of software), sometimes you "play" the game, sometimes you edit the game, sometimes you build new tools to help you edit the game, sometimes you watch others play the game.

# The System and The Scaffolding

Fundamentally, there is not a single test, certificate, proof, or "gate" that you can run to convince me that some non-trivial software program is correct.
The correctness of any complex software systems span many granuliarities, and, human written or not, misalignment can hide in any of them.
Mathematical proof, passing tests, and green gates all mean nothing if it's not what I actually _meant_.

The primary objective of "software factories" seems to be about _remove_ oneself from the details, which makes it even harder to figure out what I even want to be doing when staring through a peephole from 10 thousand feet high.
The temptation to let agents swarm out on a hunch adds a whole new dimension of yak shaving, and yet it's almost never rewarded with anything useful.

:::figure width="480" src="./vault-centering.png" alt="Cutaway drawing of a masonry vault under construction, with curved timber frames supporting the unfinished vault."
[Timber centering supporting a masonry vault, 1856](https://commons.wikimedia.org/wiki/File:Construction.voute.romaine.png) — Eugène Viollet-le-Duc, public domain.
:::

To only way to judge the correctness of general purpose software is to look, to see the software in motion under many different angles and granularities.
Notably, this is not an intelligence problem! 
It's a human problem; I just don't know what I want until I see it, and I also don't know what I _don't_ want until I see that, too.

The correctness of a system is an iterative process; its alignment must be continuous as the shape of the problem shifts.
This has always been true for any real symbolic software, but it is especially true for probabilstic software, and it's also prticularly difficult with a large, fragmented stack.

Our tools for building, interacting with, understanding software are astoundingly primitive.
I want to understand shape of software and the space of all possible software that solves all the problems I'm interested in, and then navigate that efficiently.
If software is going to run _everything_, faster than anyone can verify, how do we make sure it's the right software, built the right way?

# The Destack

Software should be open, hackable, rexmiable - and fast.
Software you can actually own, _without_ giving up on the benefits of modern stacks and the "cloud".
How do we get there, from here?
Even if we could magically replace the stack, replace it with what?

And besides, what even is the ideal "brand new stack"?
It can't be too different, or nobody - humans nor agents - would know how to use it.
Paradoxically, then, now is not the time to figure out an "ideal second system" _from scratch_; whatever follows must trace the shapes we already have (for better and worse).

Fortunately, the web is already pretty great.
TypeScript is also pretty great.
Everybody knows the web, everybody knows TypeScript, and web standards evolved over decades.
Oh, and the internet is also the largest application platform ever.
It's not _perfect_, sure, but what is?

:::figure src="./picasso-bull.jpg" alt="Eleven versions of Picasso’s bull, progressing from a detailed animal to a few essential lines."
[The Bull, 1945–46](https://drawpaintacademy.com/the-bull/) — Pablo Picasso, reproduction via Draw Paint Academy.
:::

We have long figured out that opinionated formatters are a great idea as they remove unproductive syntax debates and standardize the _appearence_ of code to just one right way.
Sure, maybe the brackets aren't _perfect_ by everyone's personal standards, but it is _standardised_ and who cares anyway, there is software to ship.
 <!--and meta-frameworks serves a similar need.-->

What formatters did to the appearance of code, we should now to do the _shape_ of code:
fully standardize far beyond syntax, language choice, and other trivialities.
We need exact schematics to fill in and follow for the p95 of solved use cases (think shadcn++), so we can then focus on the higher order bits that matter.
<!--we want _exact_ prescribed module and file layouts, type shapes, call trees, services, dependencies, entire architectures.-->

With a sufficiently standardized stack, we can finally have truly personal software that can actually be owned.
And the most pragmatic way to standardise is to pick the best of the modern web stack, package that in a common format, integrate all the boring parts of the iceberg.
And then grow from there.

<!-- TODO: demo #1 -->

That is Destack: an open source, hackable, personal software platform.
Basically, it's just a personal Git + NPM + database + compute, but with all the synchronisation, infrastructure, and just all the boring bits to make it work nicely together with unified auth, styling, models, telemetry, and such.

<!-- TODO: demo #2 -->

Inspired by game engines, Destack includes integrated tools for interactively building, understanding, and refining software .
It's as indescructible as can be, and you can host it entirely yourself, we can tunnel for you, or you could let us take care of everything.
Or any combination.

<!-- TODO: demo #2 -->

Destack launches today.
You can try it here:

<!-- TODO: CTA? -->
