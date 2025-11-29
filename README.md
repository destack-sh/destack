# Destack

Destack is a "universal software engine", a system for building fantastic full-stack systems and applications.
Build with TypeScript and the web ecosystem in one unified open source toolchain, stack and platform.
Deploy and run anywhere with the existing ecosystem as you please, free of vendor lock-in.

Destack works with the tools you like and embodies existing best practices and conventions.
Write plain TypeScript or our Destack dialect (`.ds`, much like `.tsx` or even "TypeScript++").
Everything is absurdly integrated to enable rapid end-to-end development of *fantastic* software.

## Structure

This is the fully open source monorepo of Destack containing the language, library and core platform tools:
```
language/    Language toolchain (parser, compiler, formatter, LSP, etc.)
library/     Standard library (entity, telemetry)
platform/    Platform features (CLI, IDE integrations, build plugins, etc.)
examples/    Example projects
templates/   Project templates for `destack new`
```

## Prerequisites

Destack is built using Rust and TypeScript:
- [Rust](https://rustup.rs/) (nightly-2025-11-27)
- [Bun](https://bun.sh/) (≥1.0)
- [just](https://github.com/casey/just) (command runner)
- [Python 3](https://python.org/) (optional, for code generation)

## Quick Start

```sh
# install dependencies
just install

# build everything
just build

# run tests
just test

# see all commands
just
```

## Development

```sh
just check          # check / lint everything
just fmt            # format all code
just lint           # lint all code
just language/test  # run only Rust tests
just library/test   # run only TS tests
```

See [AGENTS.md](AGENTS.md) for code style guidelines.

## License

Apache-2.0. See [LICENSE.txt](LICENSE.txt).
The Destack language, toolchain, library and core platform are fully open source.
