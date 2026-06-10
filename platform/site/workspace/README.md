# destack: universal software engine

Destack is a universal software engine for building correct, optimal, integrated software systems: a TypeScript(++) language, toolchain, VM, AOT compiler, runtime, and libraries built on top of TypeScript and the open web ecosystem.

Conceptually, Destack is the antithesis to the very idea of a "stack" - instead of wrangling many disparate languages, tools, libraries, runtimes, services, and apps, it unifies the processes of software production into one universal computing stack.

The whole engine installs with one command:

```sh
curl -fsSL https://destack.sh/install | sh
```

Verification is part of that engine rather than an ecosystem to assemble - testing, benchmarking, fuzzing, and deterministic simulation are toolchain verbs:

```sh
destack test       # typed tests, no runner to configure
destack bench      # measured work and resource units
destack fuzz       # generators and shrinking
destack simulate   # deterministic fault injection, replayable by seed
```

This workspace is a regular Destack package - a `destack.json` at the root, folders of small programs, a readme per folder - covering the design one chapter at a time:

| folder | covers |
| --- | --- |
| [`types/`](types/README.md) | precise primitives, value types, and nominality over TypeScript's type forms |
| [`expressions/`](expressions/README.md) | expression-oriented control flow, patterns, Result-first errors, comptime |
| [`runtime/`](runtime/README.md) | module metadata, typed data imports, capability policies, test/bench/simulate |
