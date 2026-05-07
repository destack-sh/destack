# Higher-Order Programming

It has been more than half a century years since [C introduced higher order programming](https://en.wikipedia.org/wiki/C_(programming_language)#History) as we know it today, yet programming is still astoundingly immature.
Our tools are a little nicer, sure: prettier GUIs, faster GCs, fatter libraries, fancier SaaS, and .. that's it - fundamentally, we're still text in, tests results with _maybe_ some logs out, and then we ship without any [_real_ confidence](https://apple.github.io/foundationdb/flow.html).
We've just grown accustomed to software being buggy, slow and fragmented.

Software "engineering" bears little resemblence to real engineering: 
we routinely fail to build trivial software correctly, and even when it works, it is incredibly inefficient, and even when it is, it is not well integrated with other software.
The inscrutability, inefficiency, and instability of software spans the entire lifecycle, and it must be solved by reimagining software production end-to-end across _all_ incidentally disparate sub-processes.

The processes and scaffolding to build and maintain the system are inextricably linked to the system itself.
Therefore, Destack is not really a "stack" at all, it is a system for understanding systems, a "meta stack".
Really, the "scaffolding" _is_ how we discover, specify and evolve the shape of the problem that the system is designed to solve.
And thus we cannot program at a higher level without precise specification across _all_ levels.

Software is very useful, we have a lot of it, and there is about to be much, much more, with exciting new possibilities to integrate probabilistic into symbolic computation.
The more we can express in software, the higher order the tasks we can program -
there is great promise in turning more things _into_ correct, optimal, integrated software systems, and we believe a universal software engine is the best way to support that.

## Why You Should Not Use Destack

Destack has been in development [for years](https://github.com/destack-sh/destack/graphs/commit-activity) and went through a _lot_ of iteration, and Destack intentionally follows good standards like TypeScript, TSX, Node-shaped APIs, and Web-shaped APIs.
However, it is still rather early, it is definitely quite radical, and there are sound arguments against the Destack-shaped "universal software engine" way:

1. **Maybe the existing stack is already good enough**: The existing "stack", its layers, and its components exist for a good reason and have withstood significant evolutionary pressure; therefore, trying to combine or even rearrange them in a substantially different way may very well turn out net negative.

2. **Maybe Destack is hard to adopt properly**: Destack is compatible with strict TypeScript-shaped source, yes, and targets modern TS/Node/Web-shaped systems, yes, but many of the most interesting features only work with "modern" TS, and especially when integrating with Destack-specific features, which require a larger shift of production processes.

3. **Maybe any ecosystem split is too expensive now**: The web ecosystem fork implied by any new language and paradigm is costly, and while transforming code is now significantly cheaper than it used to be, transforming understanding and habits and the "hard" ecosystem bits still has high friction.

4. **Maybe Destack should be more radical**: The existing (web) standards could be followed _less_ and since code transformation is now relatively cheap, and this is a unique time of disruption, maybe Destack should be even _more_ adventorous and experimtal in its design to finally do software in the "most optimal" way.

5. **Maybe Destack should be less radical**: The existing (web) standards could be followed _more_ religiously, maybe we shouldn't just pick and choose the "best" ones; they are pretty good by now, and while they're not perfect, any deviation necessarily implies imperfect transformation at some lossy edge.

6. **Maybe "TS++" is too complex and weird**: The "++" in our "TypeScript++" language might be doing too much; maybe TypeScript cannot be load-bearing in this way and just cannot structurally support it, and thus all systems programming should be left to the "real" native systems languages.

7. **Maybe Destack is too complex and weird** Following TS/TSX/Node/Web standards is nice, but there is still a novel combination of features and technologies here, and the ways of working and new processes required to make the most of Destack are unconventional and unestablished.

8. **Maybe Destack is actually good but it's too late**: Part of the value of the common software stack comes from having been around for a while and thus to have stood the test of time; any new way of doing things is thus inherently suspicious, _even if_ it is "objectively" better according to some theoretical ideal.

9. **Maybe Destack is good _today_ but "best of breed" wins eventually**: Having the vertically integrated solution be better at the beginning of a technological revolution due to the benefits of co-design is quite common, and then losing out against the benefits of specialisation is also quite common.

## Some Questions Worth Considering

Destack is pretty weird and quite unlike how software production has traditionally worked, with its own new _experimental_ way of thinking about the processes of programming. 
If you have gotten this far through reading the README, you probably have some, all of, or - maybe most curiously - none of the following questions:

1. **What even _is_ Destack? Is it a TypeScript dialect (like TSX), a JS family language (like Rescript), a JavaScript runtime (like V8), a Node runtime (like Deno), an NPM library (like vitest), a service (like Antithesis), ...?** 
All of it, none of it.
Mechanically, Destack _is_ a TSX-family language and toolchain with a VM, AOT compiler, Web/Node-shaped platform libraries, formatter, linter, rich standard libraries, and a set of common services and apps.
Conceptually, Destack is a new kind of thing: a fully integrated computing stack, a software toolkit, the building blocks you need to build your own stack.

2. **Why is Destack built on TypeScript and not some other language like Python or Rust?**
Both Python and Rust are great languages, and both fail the "universal language" test for surprisingly symmetrical reasons:
Python is pathological to optimize, but great for scripting, while Rust is great to optimize, but cumbersome for scripting.
Both Python and Rust are bad at "UI stuff", and both are structurally difficult to deploy well in a browser, which is the most popular, most universal software platform.

3. **Which JavaScript/TypeScript features are supported on Destack?** 
Destack is a TypeScript engine, not a JavaScript engine.
*Modern strict TypeScript* is fully supported, including all the fun stuff like structural interfaces and mapped types.
However, Destack is intentionally not strictly ECMAScript compliant because dynamic runtime features like `prototype`, `eval` / `Function`, dynamic `class`, and dynamic protocols like `[[Call]]` / "thenables" are forbidden (their typed forms are supported).

4. **How does Destack interact with the existing JavaScript/TypeScript/Node/web ecosystem?**
Destack runs "strict modern" TS directly, and "TS++" (`.ds` files) can transpile into `.js`/`.ts` for browsers and other JS-only runtimes. 
Destack is not a browser, and has no renderer (yet).
On the backend, Destack supports selected Node-shaped APIs, similar to other Node-derived runtimes like Bun or Deno.
Just like Destack does not intend to fully support arbitrary JS code, we also do not intend to fully support _all_ web standards.

5. **Why not support both a JavaScript "slow mode" and a TypeScript "fast mode"?**
Running "regular" Javascript _well_ is complex as it's essentially a whole second lane alongside the strict TypeScript AOT model (see [Static Hermes](https://github.com/facebook/hermes/)). 
Further, _just_ supporting untyped JS is not that useful - we would also need a full web surface for the many frontend JS libraries.
Modern backend code predominantly uses TypeScript and Node APIs already, while frontend doesn't work well natively anyway without also implementing a whole browser.

6. **Why can't we just use TypeScript/web for frontend and Rust/C++/Go for backend?** 
We can and that will continue to work pretty well, though with some friction, as the traditional distinction between "frontend" and "backend" blurs further and clients become increasingly powerful. 
Classic web UI is unfortunately very inefficient, and the existing "systems languages" are bad at the UIs we need for better software systems. 
Full-stack TS is popular for a reason: centralizing domain models and software abstractions is useful. 

7. **Why build new languages and programming systems if AI is going to be writing and maintaining code?** 
Software is more than code, and while it's possible there is a future where _no_ code is reviewed or maintained by humans, even that will take a while. 
More importantly, we need to program machines in _some_ symbolic system to control the probabilistic system, and we (probably) need _new_ programming systems to do both well.

8. **Why not abandon standards entirely and fix _all_ the problems in a whole new stack?** 
It's tempting to design the "optimal" stack, but even if AI could magically migrate everything, historically, new "big bang" systems almost always fail.
The core tension of Destack is deciding which technologies are: 
a) expressive enough to support universal software,
b) performant enough to run all software at machine speed, and 
c) familiar enough to be intuitive and reviewable.
The intersection of a, b, and c turns out web-shaped. 

9. **Why is Destack itself built on top of Rust, considering Destack and "TS++" are so great?** 
Destack is _currently_ primarily implemented in Rust, especially the language toolchain, but that is just the pragmatic bootstrapping path to eventual self-hosting.
Library, services, and apps are already written in Destack as much as possible, and we want to gradually migrate the full stack to be fully self-hosted soon(ish).
