---
title: "Introducing Destack"
subtitle: "The final stack for personal software (for real this time)."
date: "2026-10-14"
author: "Florian"
---

Software is entering its Cambrian Explosion, and - it is worth remembering - that means most specimen will go extinct, and the survivors will look very different.
Both products and processes will evolve under intense competition, with free migrations enabling the full exploration of the space of all possible software - until a new optima is found.

:::figure width="600" src="./cambrian-sea.jpg" alt="Illustration of Cambrian marine life, with Opabinia swimming above trilobites, spiny animals, and sponges."
[Cambrian marine life](https://es.knowablemagazine.org/content/articulo/alimentos-ambiente/2026/como-los-herbivoros-obtienen-aminoacidos-esenciales) — Christian Jégou, Science Source.
:::

The current stack is fundamentally broken:
We've got a thousand little software silos, each with their own slightly incompatible slice of the stack, holding together custom auth and telemetry and UI and compute and storage.
The "stack" is not really designed, coherent or integrated; it's a hodgepodge of different technologies and services where nothing _quite_ works together. 

Now, the technical debt accumulated over decades of organic software sediment growth is coming due.
Probabilistic computing is finally here - we _could_ let agents contribue to and become part of all software, not not by tacking them on top of and next to one immovable, opaque stack that rejects integration and modification at every layer.

> To put it quite bluntly: as long as there were no machines, programming was no problem at all; when we had a few weak computers, programming became a mild problem, and now we have gigantic computers, programming has become an equally gigantic problem.
>
> — Edsger Dijkstra, *The Humble Programmer* (1972)

Half a century after the last software crisis, our computing stack is once again buckling under the weight, volume and speed of a more powerful machine.
Once more, we will have to retrace from the beginning, reconsider what programming even means, and reshape what software systems should look like for the 21st century.

# Higher Order Programming

The history of programming has been one of increasing levels of abstraction: from handcrafting gears, to wiring up vacuum tubes, to punching cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script however it likes.
And that's great.

Climbing the ladder of abstraction yields more output for every bit of input.
We gradually remove ourselves from the cumbersome burden of having to actually spell out _exactly_ what we want the machine to be doing: which wires? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

:::figure width="440" src="./babbage-engine.jpg" alt="The 1832 demonstration portion of Babbage’s Difference Engine No. 1, with its columns of brass gears and hand crank."
[Difference Engine No. 1, 1832](https://commons.wikimedia.org/wiki/File:Babbages_difference_engine_1832.jpg) — Sebastian Wallroth, public domain.
:::

Programming, then, is just reified problem solving: iterating, thinking, working to understand the shape of a problem and then specifying its solution in some repeatable form.
We used to solve software-shaped problems with artisanal human-directed next-character-prediction of symbolic code, but really, it doesn't have to be any formal language at all.

The idea of programming beyond code is as old as code itself, and there is broad and successful prior art on "higher order programming".
Most prominently, game developers have been doing this for _decades_:
early on, game development was also a complete schlep, and only a tiny guild of brilliant nerds could pull off presentable commercial games.

Initially, to build a game, _everyone_ had to write their own graphics, networking, scripting, asset pipelines, editors - a whole engine for every game, on top of the actual game!
Then, eventually, we figured out how to package the hard bits into reusable components and evolve more complete, higher level packages of reusable software components we call a "game engine".

:::figure width="480" src="./ibm-extra-engineers.jpg" alt="IBM’s 1951 advertisement, 150 Extra Engineers, showing rows of engineers doing calculations."
[150 Extra Engineers, 1951](https://commons.wikimedia.org/wiki/File:IBM_150_Extra_Engineers_1951.jpg) — IBM.
:::

Importantly, modern game engines go _far_ beyond traditional "software frameworks" like Django or Rails or even any combination thereof. 
The useful kind of game engine is an integrated software platform: a runtime, an editor, tools, version system integrations, netcode, deployment platform, and myriad affordances for prototyping, iterating, refining, and shipping games.

Like most new abstractions, game engines _initially_ didn't provide quite the same level of control as the underlying level.
Over time, however, building with a game engine became so much more productive that it enabled a new scale of project, and empowered a new type of contributor - designers, writers, and artists, could now contribute _directly_.

Over all these years games never "superseded" code as such, even as a lot of code was abstracted away for use cases that previously required it.
It's still important, but, thanks to modern game engines, you can now build and ship real commercial games without thinking in code.

# The Software Engine

Software at large is now sprinting towards the same tradeoffs that game developers have figured out over decades.
We know, roughly, the principal components, but they're scatted and rebuilt a million ways.
Now, we also need a "software engine": a "game engine"-like abstraction of _integrated_ "platform software" building blocks for a new level of "userland software". 
<!--Sometimes, when building a game, you playtest the game, sometimes you edit the game, sometimes you build new tools to help you edit the game - but it's all part of _one_ integrated process.-->

Fundamentally, there is not a single test, certificate, proof, or "magic gate" that you can run to convince me that some non-trivial software program is correct.
The correctness of any complex software systems spans many granularities, and mathematical "proof", green tests, smoke tests, and passing gates all mean nothing if it's not what I actually _meant_.

Oddly, the primary objective of "software factories" seems to be about _removing_ oneself from _all_ the details without any sufficient higher order specification to anchor this new higher order process.
In frighteningly short order, steering exclusively through the peephole of a chat window makes it almost impossible to figure out where to go next and how to get there.

:::figure width="480" src="./vault-centering.png" alt="Cutaway drawing of a masonry vault under construction, with curved timber frames supporting the unfinished vault."
[Timber centering supporting a masonry vault, 1856](https://commons.wikimedia.org/wiki/File:Construction.voute.romaine.png) — Eugène Viollet-le-Duc, public domain.
:::

The only way to judge correctness for general purpose software - that is: does it do what it should? - is to look, to see the software in motion under many different angles and granularities.
Notably, this is not an intelligence problem at all - it's a human problem, and a real world integration problem; I just don't know what I want until I see it.

Despite half a century of development, our tools for building and understanding software systems are surprisingly primitive.
We _still_ struggle with "works on my machine", and reproducing observed issues in the "laboratory" is virtually impossible.
If software is going to run _everything_, faster than anyone can verify, how do we make sure it's the right software?

# Personal Software, Seriously

Software was never meant to be a thousand tabs with their own fiefdoms.
Of course, the scale of teechnology an d
<!--The pioneers meant for software to be open, hackable, remixable - a malleable medium that could fluently adapt to user needs without being locked into any specific application, data format, or service provider.
And while hardware has advanced far beyond even their wildest dreams, a much duller and infinitely fragmented software reality has settled in.-->
And indeed, the dream of "malleable software" and "end-user programming" has been tried valiantly again and again - only to die again and again.
Sort of.
The closest to malleable software we have today are constrained sandboxes like Excel and Notion, which are useful, but not _general_, and so we are stuck with a thousand little rented software silos.

<!--:::figure width="600" src="./tiny-glade.gif" alt="In Tiny Glade, drawing a path through a wall creates an archway automatically."
[Path editing in Tiny Glade](https://www.youtube.com/watch?v=CdWpq2efN8Y) — Pounce Light, trailer excerpt.
:::-->

<!--Yet, our new class of machine user demands the broadest possible access to the entire software stack, with as many degrees of freedom as we can safely provide, up and down and left and right. -->
<!--Agents are on pace to outnumber human users by multiple orders of magnitude very soon, but they are awkwardly caged in by last century's software stack.-->

So far, the cardinal sin of "malleable software" has been that nobody wants to build *and maintain* their own software.
Custom software is enticing, and integrated software sounds lovely, but it's deceptively difficult: it never works _quite_ as well as the off-the-shelf alternative, and there was little benefit to controlling your stack anyway.

Now, the equation has flipped: building is cheap, and the lack of integration is expensive.
How do you join "reminders in Notion" with "leads in Salesforce" and "events in Outlook"?
It simply does not compute - the depth of integration required is impossible to achieve by merely tying together slices of the old stack.

:::figure width="640" src="./wright-flyer-wind-tunnel.jpg" alt="A full-size Wright Flyer replica mounted on a test stand inside the Ames wind tunnel, with two engineers standing beside it."
[Wright Flyer replica in the Ames wind tunnel](https://www.nasa.gov/image-article/wright-flyer/) — NASA, public domain.
:::

Of course, the current software crisis goes well beyond a mere "lack of integration":
it's not just that software doesn't work well _together_, it's that it doesn't work well _at all_.
If software is solved, why is there still so much bad software?
How do we put the "engineering" into "software engineering"?

There are all these amazing new probabilistic computing capabilities to embed deep and wide into our symbolic stack, but where is all the automation? 
Why aren't all form fields automatically pre-filled?
Why doesn't _every_ textbox have autocomplete based on my own profile..?

<!--The brute acceleration of old processes with "self-driving factories" will not magically yield a better stack with better software, just _more_ of the same old stack with the same old problems - except with less oversight, and more impact.-->
<!--No.-->
The precise manufacture of malleable software requires _new_ integrated processes, and those processes need to span the entire lifecycle beyond just production.


# The Destack

Software should be open, hackable, remixable - a stack you can own, without the hassle of traditional software ownership.
To make it so, we need a "software engine": a stable platform of _integrated_ building blocks designed for iteration and higher order programming of fluid, just-in-time software _without_ giving up on the benefits of modern stacks and the cloud.

So.
How do we get there, from here?
What even is this ideal "final stack"?
It can't be too different, or nobody - human or agent - would know how to use it.
Paradoxically, now is _not_ the time to figure out an "ideal second system" _from scratch_, the final stack must trace the old stack (for better and worse).

:::figure src="./picasso-bull.jpg" alt="Eleven versions of Picasso's bull, progressing from a detailed animal to a few essential lines."
[The Bull, 1945–46](https://drawpaintacademy.com/the-bull/) — Pablo Picasso, reproduction via Draw Paint Academy.
:::

Fortunately, the web is already pretty great.
TypeScript is also pretty great.
Everybody knows the web, everybody knows TypeScript, and web standards have evolved over decades.
Oh, and the internet is also the largest application platform ever.
It's not _perfect_, sure, but what is?

Instead of building a theoretically perfect second system, we can just _standardize_ - to an absurd degree - around the best parts of the modern stack, and integrate them very deeply.
We have long figured out that opinionated formatters are a great idea, so now we need to  exact schematics to fill in and follow for the p95 of solved use cases (think shadcn++), so we can then focus on the higher order bits that matter.
<!--we want _exact_ prescribed module and file layouts, type shapes, call trees, services, dependencies, entire architectures.-->

<!-- TODO: demo #1 -->

That is Destack: an open source, hackable, personal software platform.
Basically, think of Destack as a personal Git + NPM + SQlite + worker with all the auth, schemas, synchronisation, telemetry, infra, including all the boring bits to make it work properly.
Completely standardised, as much "set and forget" by default as possible.

<!-- TODO: demo #2 -->

Following the game engine model, Destack integrates libraries and tools for interactively building, understanding, and refining software products, all based on modern best practices.
It's as indestructible as can be, and you can host it entirely yourself, we can tunnel for you, or you could let us take care of everything.
(Or any combination.)

<!-- TODO: demo #2 -->

Destack launches today.
You can try it here:

<!-- TODO: CTA? -->
