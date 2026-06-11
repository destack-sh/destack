# destack: universal software engine

Destack is a universal software engine for building correct, optimal, integrated software systems: a **TypeScript(++) language, compiler, toolchain, VM, runtime**, and libraries built on top of TypeScript and the open web ecosystem.

Mechanically, Destack is an integrated stack for building software systems extremely well, but conceptually, Destack is the antithesis to the very idea of a "stack": instead of wrangling many disparate languages, tools, libraries, runtimes, services, and apps, it unifies the processes of software production into one universal computing stack:

 - **Destack Language**: TypeScript(++) toolchain, VM, AOT compiler, runtime.
 - **Destack Library**: Rich standard library for most things most software needs.

The whole engine installs with one command:

```sh
curl -fsSL https://destack.sh/install | sh
```

The general philosophy of Destack is to finally turn software engineering into real engineering by integrating all the disparate pieces into one integrated computing stack:

```sh
destack fmt        # format 
destack lint       # lint, including custom lints
destack check      # type-check and lint
destack run        # runs with granular permissions
destack test       # classic tests
destack bench      # measured work and resource units
destack fuzz       # generators and shrinking
destack simulate   # deterministic fault injection, replayable by seed
```

---

This workspace is a (mostly) regular Destack package - a [`destack.json`](destack.json) at the root, folders of small programs, a readme per folder - covering the design one chapter at a time:

| folder | covers |
| --- | --- |
| [`types/`](types/README.md) | precise primitives, value types, and nominality over TypeScript's type forms |
| [`expressions/`](expressions/README.md) | expression-oriented control flow, patterns, Result-first errors, comptime |
| [`runtime/`](runtime/README.md) | module metadata, typed data imports, capability policies, test/bench/simulate |
