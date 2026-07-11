---
title: Overview
description: The language, toolchain, runtime, and the case for an integrated computing stack.
order: 10
---

# Overview

The Destack language (`.ds`) is a superset of a strict subset of TypeScript, with true native AOT compilation, a fully integrated toolchain, and standard JavaScript and TypeScript output targets.
Strict "modern" TypeScript code within this subset "just works", but Destack has absolutely **no JavaScript or NPM interoperability**  (see [comparison](comparison.md)).

We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated computing stack, and thus by "language" ("TypeScript++") we mean much more than "just" the programming language itself: a language, a runtime, a toolchain, plugins, libraries, and ultimately, a way of programming.
It's all connected, and to leave out a part would be to betray the whole, which is why we need to begin with an _actual_ programming language.

## Universality and Completeness

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to get any code out the door, and yet confidence and performance have plummeted.
We can do better, but not by adding _more_ and more inscrutable pieces.

The best possible stack would be fully integrated across the language itself, the toolchain, the runtime, and basically anything that touches the software stack.
To be fully integrated, we need a base programming language that can actually run all modern software efficiently across all relevant target platforms.

Only TypeScript is seriously close to being a universal software foundation: it is the most popular and familiar programming language, it runs _directly_ on the web, and the web is the most ubiquitous software platform.
The TypeScript ecosystem has good - if not perfect - conceptions of answers to all modern software needs, from great developer tools to rich interactive frontends to reasonably performant backends.

Disregarding the legacy JavaScript baggage, modern TypeScript is surprisingly close to a fully statically compilable language - indeed, most browsers retrofit compilation internally already based on this assumption.
Embracing TypeScript and "the web ecosystem" lets us build a new toolchain that covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be made to run as fast as the machine allows.
