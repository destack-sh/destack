# Destack: Universal Software Engine

Destack is a universal software engine for building correct, optimal, integrated full-stack systems.
Build on TypeScript and the web ecosystem with a unified open source toolchain, stack and platform.
Own your software and run it anywhere in one integrated open source stack:

 - **Destack Language**: TypeScript++ with opt-in extensions for correctness, ergonomics, and performance (with optional `.ds` files like we have `.tsx`). 
 Valid TypeScript is valid Destack, so adopting `.ds` is incremental.
See [language/DESIGN.md](language/DESIGN.md) and [language/SPECIFICATION.md](language/SPECIFICATION.md).

 - **Destack Library**: Standard library for most things most software needs. Fully integrated, batteries-included components for every layer of the stack and every part of the software lifecycle.

 - **Destack Platform**: Integrated platform to support your software - the CLI, IDE integrations, build plugins, debugging, analytics, deployment tools. 
 And the platform to integrate Destack software, fully integrated with the same language and tools.

**Destack is designed as an integrated system**, **but you *can* pick and choose any pieces you like.**
You are free to use plain TypeScript, pick any of the libraries, or plug into the platform from a different system entirely.

## Examples

 - TODO #Incomplete: examples

## Getting Started

 - TODO #Incomplete: getting started (`bun i destack`, `curl destack.sh/install`, ..)
 - Join the [Discord](https://discord.gg/xUFQ45TWYd)

## Higher-Order Software

We're very early in software as an industry.
Software is broken, slow, and hard to build right.
Computers are miraculously fast, yet software feels slow and clunky.
Destack aims to make building correct, optimal, integrated software the obvious default.

TypeScript is a language for describing *the shape of datae*.
Destack is a system for describing *the shape of software*.
TypeScript has `.ts`, `.d.ts`, and `.tsx`, Destack brings `.ds` and `.d.ds` into the same codebase.
The fully integrated library and platform let us extend the TypeScript phiolosphy for entire software systems. 

The best programing language is the one that fits the problem.
And this "language" encompasses supporting libraries, the platform and its ecosystem.
The more we can express in one unified system, the more the toolchain can verify, optimize, and assist.
Higher-order software raises the level of abstraction of what software can express *reliably*.

## Development

> **You don't need to do any of this to *use* Destack!** Go to [Getting Started](#getting-started) to install and run Destack.

Destack is in very active development with a singular focus: a fully integrated software stack for optimal, correct, integrated software. 
We welcome feedback, issues, ideas, and small fixes, but please reach out for any non-trivial contributions (see [Contributing](CONTRIBUTING.md)).

### Setup

Destack is *developed* using Rust and TypeScript. 
To build it locally you will need at least `bun`, `cargo` and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see `rust-toolchain.toml`)
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [Python](https://python.org/): Scripts and codegen utilities (optional)
- [just](https://github.com/casey/just): Scripts and command runner

We use `justfile`s: 
```sh
just install 		# setup everything
just check          # check & lint everything
just fmt            # format all code
just lint           # lint all code
```

### Structure

This is the open source monorepo containing the language, library, and platform core:

| Directory | Description | README |
|--------------|----------------------------------------------------------------|-------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.)    | [language/README](language/README.md)  |
| `library/`   | Standard library (entity, telemetry, math, physics, UI, etc.)  | [library/README](library/README.md)   |
| `platform/`  | Platform features (CLI, IDE integrations, build plugins, etc.) | [platform/README](platform/README.md)  |
| `examples/`  | Example projects                                               | [examples/README](examples/README.md)  |
| `templates/` | Project templates for `destack new`                            | [templates/README](templates/README.md) |


## License

**MIT license**. See [LICENSE.txt](LICENSE.txt) for details.
The Destack language, toolchain, library and platform core are fully open source.
