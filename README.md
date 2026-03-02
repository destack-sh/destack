<div align="center">
<p>
    <img src="./.github/assets/banner.svg" width="960" alt="Destack banner">
</p>

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

Open source TypeScript(++) toolchain, VM, AOT compiler, runtime, library, services, and apps.

<p>
    <a href="VERSION"><img src="https://img.shields.io/badge/version-0.55.2-2ea44f?style=for-the-badge" alt="Version"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/nightly.yml"><img src="https://img.shields.io/github/actions/workflow/status/destack-sh/destack/nightly.yml?branch=main&logo=github&style=for-the-badge" alt="Nightly"></a>
    <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="MIT License"></a>
    <a title="Discord" target="_blank" href="https://discord.gg/xUFQ45TWYd"><img alt="Chat with Destack people on Discord" src="https://img.shields.io/discord/1079840654466752606?label=Discord&logo=discord&logoColor=white&style=for-the-badge"></a>
</p>

</div>

---

## The Destack

**Destack is a universal software engine with a language, runtime, libraries, services, and apps built on top of TypeScript and the open web ecosystem.**
Conceptually, Destack is the antithesis to the very idea of a "stack":
instead of wrangling many disparate cast-iron languages, tools, libraries, approaches, services, and apps, Destack unifies the processes of software development into _one_ malleable computing stack:

- [**Destack Language**](language/README.md): TypeScript(++) toolchain, VM, AOT compiler, runtime.
- [**Destack Library**](library/README.md): Standard library for most things most software needs.
- [**Destack Services**](service/README.md): Runtime, infra and developer services.
- [**Destack Apps**](app/README.md): First-party applications and developer tools.
- [**Destack Bridge**](bridge/README.md): External-facing SDKs, editor integrations, and host tooling bridges.
- [**Destack Templates**](template/README.md): Ready-to-clone starter kits for common use cases.

Aspiritionally, **Destack is a meta framework for building and customizing your _own_ stack with foundational building blocks**.
Destack is more of a "software factory toolkit" than "ready-to-wear software"; it is optimized for developers building their own software processes in one correct, optimal, integrated system.
Therefore, while Destack is designed as an integrated system, you are encouraged to pick and adapt just the components you need.

---

## Higher-Order Programming

It has been over 50 years since C introduced higher order programming as we know it today, yet programming is still astoundingly immature.
We routinely fail to build trivial software correctly, and even when it works, it is incredibly inefficient, and even when it is, it's not well integrated with other software.

Software is very useful, we have a lot of it, and there is about to be much, much more.
There are even new exciting possibilities to marry symbolic and probabilistic computation.
But we believe the deep opaqueness, inefficiency, and fragmentation of software can only be solved by reimagining the full software process; in the limit, that means unifying the disparate parts that have remained separate purely for historical reasons.

The more we can express in software, the higher order the abstractions we can program.
In the beginning, software was the digital shadow of "real" systems, but done correctly, software is an enabling technology for new systems that were previously impossible.
There is great promise in turning more things _into_ correct, optimal, integrated software systems, and we believe a universal software engine is the best way to support that.

---

## Why You Should Not Use Destack

> [!WARNING]
> **Destack is an alpha-stage, _experimental_ computing stack.**
> Things may change or break or disappear entirely.

Destack has been in development for years and went through a _lot_ of iteration, and Destack intentionally follows known good standards like TypeScript, TSX, Node and Web-shaped APIs.
However, obviously, it is still rather early, it is definitely quite different, and there are sound arguments against the Destack-shaped "universal software engine" way:

1. **Maybe the existing stack is already good enough**: The existing "stack", its layers and components exist for a good reason and have withstood significant evolutionary pressure, thus trying to combine or even rearrange them in a very different way may very well turn out net negative.

2. **Maybe Destack is too Destack-special**: Destack is compatible with JS/TS, yes, and runs modern TS, yes, but many of the most interesting features only work with "modern" TS, and especially when integrating with more of the "destack" stack, which is a bigger shift.

3. **Maybe any ecosystem split is too expensive now**: The web ecosystem fork implied by any new language and paradigm is costly, and while transforming code is now significantly cheaper than it used to be, transforming understanding and habits and the "hard" ecosystem bits is still highly non-trivial.

4. **Maybe Destack should be more radical**: The existing (web) standards could be followed _less_ and since code transformation is now relatively cheap, and this is a unique time of disruption, maybe Destack should be even _more_ adventorous and experimtal in its design to finally do software in the "most optimal" way.

5. **Maybe Destack should be less radical**: The existing (web) standards could be followed _more_ religiously, maybe we shouldn't just pick and choose the "best" ones; they are pretty good by now and while they're not perfect, any deviation necessarily implies imperfect transformation at some lossy edge.

6. **Maybe "TS++" is too complex and weird**: The "++" in our "TS++" language is trying to do too much; maybe TypeScript is not meant to be load-bearing in this way and just cannot be, and all systems programming should be left to "native" systems languages.

7. **Maybe Destack is too complex and weird** Following TS/TSX/Node/Web standards is nice, but there is still a novel combination of features and technologies here, and the ways of working and processes required to make the most of Destack are unconventional.

8. **Maybe Destack is actually good but it's too late**: A substantial part of the value of the common "stack", much like with other hard-to-evaluate technologies, comes from having been around for a while and thus to have stood the test of time; any new way of doing things is thus inherently suspicious, _even if_ it is "objectively" better overall.

9. **Maybe Destack is good _today_ but eventually "best of breed" will win again**: Having a "fully integrated" solution win out over special solutions at the onset of a technological change due to the benefits of integration is quite common, and then losing out against the benefits of specialisation is also quite common, which then forces annoying userland churn.

### If you _really_ insist on using Destack

We tried to warn you:
- Install Destack via `curl -fsSL https://destack.sh/install | sh` or `npm i -g @destack/cli`.
- Create a new Destack app with `npm create destack@latest my-destack-app` or `bun create destack my-destack-app`.

---

## Some Questions You Should be Asking

Destack is pretty weird and quite unlike how software development has traditionally worked, with its own new _experimental_ way of thinking about programming overall. 
If you have gotten this far through reading a README, you probably have some, all of, or - maybe, most curiously - none of the following questions.
We answered the void anyway:

1. **What even _is_ Destack, exactly? Is it a TypeScript dialect (like TSX), a whole new language (like Rescript), a JavaScript runtime (like V8), a Node runtime (like Deno), an NPM library (like vitest), a service, an app, a CLI, ..?** 
All of it, and none of it. 
Mechanically, Destack _is_ a TypeScript toolchain with a VM, AOT compiler, Node-like runtime, formatter, linter, rich standard libraries, and a set of common services and apps. 
Conceptually, Destack is somewhat novel; nobody has _really_ tried to integrate this deeply since the likes of Pharo, GT or SmallTalk.
Maybe for good reason. We will see. 

2. **Why is Destack built around TypeScript? Why not some other language, like Python or Rust?**
Both Python and Rust are great languages that we like very much, and both fail the "universal language" test for surprisingly symmetrical reasons:
Python is pathological to optimize, but is great for scripting, while Rust is great to optimize, but awful for scripting.
Both Python and Rust are bad at "UI stuff", and both are structurally difficult to run well in a browser, which is the universal application platform.

3. **Which JavaScript/TypeScript features does Destack forbid to enable real AOT compilation and all the other fancy stuff? Do I need to rewrite everything?** 
All modern TypeScript features are supported, but highly dynamic features like `prototype`, `eval`, `Function`, dynamic `class`, etc., are explicitly, and thus also everything that depends on them.
If you're used to writing strict modern TypeScript, you're almost certainly already Destack-compliant language-wise. 
If you're building Node-shaped backend services, you might not even need to change anything (depending on what your dependencies do).

4. **Why not support both a JavaScript "slow mode" and a TypeScript "fast mode" (like Static Hermes)?**
Language-wise, yes, that would help much more of the existing JS-first ecosystem to run directly on Destack.
However, it would add significant complexity and a whole second execution path, diminishing the benefits of full-stack integration.
Moreover, _just_ supporting untyped JS is only partly useful, we would also have to support significantly more of the "legacy" web surface, especially for the many web-oriented libraries.
In general, most modern "backend" code already uses TypeScript and Node-ish APIs, while "frontend" stuff doesn't work well natively anyway without a rewrite, so this seems like a reasonable tradeoff pointw.

5. **How does Destack interact with the existing JavaScript/TypeScript/Node/web ecosystem?**
Destack runs TS directly, and TS++ (`.ds` files) can be transformed into `.js`/`.ts` for consumption by browsers and server runtimes. 
On the backend, Destack also supports Node-shaped APIs, so - like with other Node-compatible runtimes - if you stay within the Node APIs, you remain fully compatible.
Going the other way, Destack does _not_ fully support arbitrary JS/TS code *on the native path* (incl. VM and runtime).

6. **What about _Other Technology_? Other Technology already exists, and is proven, and everyone knows it.** 
Yes, excellent point. 
There is substantial prior art along most axis that Destack covers: web-shaped runtimes (every browser), JS/TS VMs (V8/JSC/Hermes), JS runtimes (Node/Bun/Deno), JS/TS libraries (all of NPM), formatters, linters, and on and on.
One of the challenges in pushing technology forward is picking the right point to be novel enough to be interesting while being familiar enough to be useful.

7. **Why can't we just keep using TypeScript for front-end and Rust/C++/Go/whatever for back-end systems-y stuff?** 
We can, and that has worked quite well for a long time, and will continue to work pretty well.
Unfortunately, the generality and legacy baggage of classic web frontends impose a bad performance ceiling.
However, there is a reason full-stack TS is so popular: centralizing domain models and programming concepts is very useful. 
And, perhaps more importantly, having simple scripting _and_ rich UI _and_ systems features in one stack enables much nicer visualisations and tooling.

8. **What's the point of building new languages and programming systems if AI is going to be writing and maintaining code?** 
Software is more than code, and while it's possible there is a future where _no_ code is reviewed or maintained by humans, even that will take a while. 
More importantly, we need to program machines in _some_ symbolic system to control the probabilistic system. 
What those systems look like _exactly_ is up for debate, but we reckon it won't be _completely_ different from the same nouns and verbs we already know, primarily because we already know them.

9. **Wy not abandon existing standards entirely and fix _all_ the problems, considering we're rewriting the stack anyway?** 
Yeah, it is tempting to go ahead and design the "theoretically optimal" stack, in the hopes that AI will just make it trivial to migrate and all the "optimal" concepts will reasonate immediately.
However, one of the core tensions in designing Destack is deciding which standards and approaches are good for a universal stack, and "good" means all of: 
a) expressive enough to support everything we might want, 
b) performant enough to run everything as fast as possible, and 
c) familiar enough to be immediately usable and reviewable.
The intersection of a, b, and c turns out to be surprisingly web-shaped. 

10. **Why is Destack itself built on top of Rust, considering Destack and "TS++" are so great?** 
Destack is _currently_ primarily implemented in Rust, especially the language toolchain, but that is just the pragmatic bootstrapping path to eventual self-hosting.
Library, services, and apps are already (mostly) written in Destack itself.

---

## Platforms and Targets

Destack supports the web, of course, and also runs natively on Linux, macOS, and Windows as Tier 1 targets, with mobile (iOS, Android) still coming online.
See [TARGETS.md](TARGETS.md).

| Tier | Target triples |
|------|----------------|
| **Tier 1: full support** | `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` |
| **Tier 2: pending support** | `aarch64-apple-ios`, `aarch64-linux-android` |
| **Tier 3: eventual support** | `wasm32-wasip1` |

## Contributing

Destack is in [very active development](CONTRIBUTING.md) with a singular focus: a fully integrated software stack for optimal, correct, integrated software systems.
We welcome feedback, issues, ideas, and small fixes, but please reach out first for non-trivial contributions.
Large unsolicited PRs will be closed.
See [TESTING.md](TESTING.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

The Destack language, toolchain, library, service, app, bridge, and template are fully open source under the MIT license.
See [LICENSE.txt](LICENSE.txt).

Destack includes components licensed, vendored and integrated from third parties, which come with their own licenses including the Apache-2.0 (WITH LLVM-exception) license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
