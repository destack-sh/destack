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
We've got a thousand little software silos, each with their own slightly incompatible slice of the stack and terms and standards.
The "stack" was never really designed, re-designed or integrated; it's a hodgepodge of different technologies and services where nothing _quite_ works together. 

Now, the technical debt accumulated over decades of organic software growth is coming due.
Probabilistic computing is finally here: we _could_ let agents contribue to and become part of all software, but we can't, because we're stuck on top of an immovable, opaque stack that rejects integration and modification at every layer.

> To put it quite bluntly: as long as there were no machines, programming was no problem at all; when we had a few weak computers, programming became a mild problem, and now we have gigantic computers, programming has become an equally gigantic problem.
>
> — Edsger Dijkstra, *The Humble Programmer* (1972)

The familiar features that make modern software "modern" - cloud based, SaaS, isolated vendors, deep dependency chains - also make it downright hostile to agents. 
Our new class of machine user requires the broadest possible access to the entire software stack, with as many degrees of freedom as we can safely provide, up and down and left and right. 

So, half a century after the last software crisis, our computing stack is once again buckling under the weight, volume and speed of a more powerful machine.
Once more, we get to retrace from the beginning, reconsider what programming even means now, and reshape what the shape of 21st century software should be.

# Higher Order Programming

The history of programming machines to do our bidding is one of monotonically increasing levels of abstraction: from handcrafting mechanical gears, to wiring up vacuum tubes, to punching paper cards, to coding assembly, to writing C, to programming Java, to scripting Python, to asking an LLM to script in whatever way.

Climbing the ladder of abstraction yields more output for every (human) bit of input.
We gradually remove ourselves from the cumbersome burden of having to spell out _exactly_ what we want the machine to be doing: which gears? which wires? which bits? which registers? what memory? what computer? _where_ computer? _when_ computer?

:::figure width="440" src="./babbage-engine.jpg" alt="The 1832 demonstration portion of Babbage’s Difference Engine No. 1, with its columns of brass gears and hand crank."
[Difference Engine No. 1, 1832](https://commons.wikimedia.org/wiki/File:Babbages_difference_engine_1832.jpg) — Sebastian Wallroth, public domain.
:::

Programming, then, is just reified problem solving: iterating, thinking, working to understand the shape of a problem and then specifying its solution in some repeatable form.
We used to solve software-shaped problems with artisanal human-directed next-character-prediction of symbolic code, but really, it doesn't have to be any formal language at all.

There was programming before code, there is programming after code, and the programming _outside_ code has rich prior art around "higher order programming".
_Game_ developers have perfected multimodal, multi-disciplinary software production over decades:
early game development was the original schlep, and only a tiny guild of brilliant nerds managed to produce presentable commercial games.

To build a game, _everyone_ had to write their own graphics, networking, scripting, asset pipelines, editors - a whole engine for every game, on top of the actual game!
Then, we figured out how to package the hard bits into reusable components and more complete, higher level packages of reusable software components we call a "game engine".

:::figure width="480" src="./ibm-extra-engineers.jpg" alt="IBM’s 1951 advertisement, 150 Extra Engineers, showing rows of engineers doing calculations."
[150 Extra Engineers, 1951](https://commons.wikimedia.org/wiki/File:IBM_150_Extra_Engineers_1951.jpg) — IBM.
:::

Importantly, modern game engines go _far_ beyond traditional "software frameworks" like Django or Rails or any specific cloud service. 
To best kind of game engine is an integrated software platform: a runtime, an editor, tools, VCS integrations, netcode, deployment platform, and myriad affordances for prototyping, iterating, refining, and shipping games.

Like most new abstractions, game engines _initially_ didn't provide quite the same level of control as the underlying level.
Over time, however, building with a game engine became so much more productive that it enabled a new scale of project, and empowered a new type of contributor - designers, writers, and artists, could now contribute _directly_.

Over all these years games never "superseded" code as such, even as a lot of code was abstracted away for use cases that previously required it.
Code is still there, it's still relevant, but, thanks to modern game engines, you can now build and ship real commercial games without thinking in code.

# The Software Engine

General purpose software is now discovering the same tradeoffs that game engines have figured out over decades.
We know the principal components, but they're scatted and rebuilt a million ways, and it follows that we also need a "software engine": an _integrated_ "platform software" building blocks for a new level of "userland software". 

Despite half a century of development, our tools for building and understanding software systems are surprisingly primitive.
We _still_ struggle with "works on my machine", and reproducing observed issues in the "laboratory" is virtually impossible.
If software is going to run _everything_, faster than anyone can verify, how do we make sure it's the right software?

Oddly, the primary objective of "software factories" seems to be _removing_ oneself from the process without any sufficient higher order specification to anchor this new higher order programming.
In frighteningly short order, steering only through the peephole of a chat window makes it almost impossible to figure out where to go next and how to get there.

:::figure width="640" src="./wright-flyer-wind-tunnel.jpg" alt="A full-size Wright Flyer replica mounted on a test stand inside the Ames wind tunnel, with two engineers standing beside it."
[Wright Flyer replica in the Ames wind tunnel](https://www.nasa.gov/image-article/wright-flyer/) — NASA, public domain.
:::

The software _factory_ requires a software _engine_, because the precise manufacture of quality software requires _new_ integrated processes, and those processes need to span the entire lifecycle.
It is insufficient to accelerate a trivial part of the process

Fundamentally, there is not a single test, certificate, proof, or magic gate that you can run to convince me that some non-trivial software program is correct.
The correctness of any complex software systems spans many granularities, and mathematical "proof", green tests, smoke tests, and passing gates all mean nothing if it's not what I actually _meant_.

The only way to ensure correctness for general purpose software - that is: does it do what it should? - is to look, to see the software in motion under many different angles and granularities.
Notably, this is not an intelligence problem at all - it's a human problem, and a real world integration problem; I just don't know what I want until I see it.

:::figure width="600" src="./tiny-glade.gif" alt="Reshaping walls, gates and buildings in Tiny Glade."
[Building in Tiny Glade](https://store.steampowered.com/app/2198150/Tiny_Glade/) — Pounce Light, release date trailer excerpt.
:::

The continuous iterative process of _shaping_ is critical not just for "malleable" software but for all software.
Software development - from databases to web apps to spreadsheets - spans one continuous spectrum of reified program solving that has been separated into rigid worlds only by historical happenstance.

So far, the cardinal sin of "malleable software" has been that nobody wants to build *and maintain* their own software.
Now, the equation has flipped: building is cheap, and the lack of integration is expensive.
How do you join "reminders in Notion" with "leads in Salesforce" and "events in Outlook"?
It simply does not compute.

There are amazing new probabilistic computing capabilities to embed deep and wide into all software, but we just have MCPs and chatboxes in the sidebar. 
Forms don't even _prefill_, software is not adaptive, apps are not magic - where is all the automation?
That is: where is the higher order software?

# The Destack

Higher order programming requires a higher level of abstraction that can be wielded reliably
Software should be open, hackable, remixable - a stack you can own, without the hassle of traditional software ownership.
To make it so, we need a "software engine": a stable platform of _integrated_ building blocks designed for iteration and higher order programming of fluid, just-in-time software _without_ giving up on the benefits of modern stacks and the cloud.

:::figure src="./picasso-bull.jpg" alt="Eleven versions of Picasso's bull, progressing from a detailed animal to a few essential lines."
[The Bull, 1945–46](https://drawpaintacademy.com/the-bull/) — Pablo Picasso, reproduction via Draw Paint Academy.
:::

<!-- TODO: demo #1 -->

That is Destack: an open source, hackable, personal software platform.
Basically, think of Destack as a personal Git + NPM + SQlite + worker with all the auth, schemas, synchronisation, telemetry, infra, including all the boring bits to make it work properly.
Completely standardised, as much "set and forget" by default as possible.

<!-- TODO: demo #2 -->

Following the game engine model, Destack integrates libraries and tools for interactively building, understanding, and refining software products, all based on modern best practices.
It's as indestructible as can be, and you can host it entirely yourself, we can tunnel for you, or you could let us take care of everything.
(Or any combination.)

<!-- TODO: demo #2 -->

<!-- TODO: CTA? -->
