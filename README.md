# Destack

Destack is a universal software engine for building correct, optimal, integrated full-stack systems.
Build with TypeScript and the web ecosystem with a unified open source toolchain, stack and platform.
Own your software and run it anywhere in one integrated stack:

 - **Destack Language**: TypeScript++ for correctness, ergonomics, and performance (with `.ds` much like `.tsx`). 
 Valid TypeScript is valid Destack.
See [language/DESIGN.md](language/DESIGN.md) and [language/SPECIFICATION.md](language/SPECIFICATION.md).

 - **Destack Library**: Standard unified modules for everything software needs across the stack from APIs and UIs to DBs and telemetry.
Batteries included, but not required.

 - **Destack Platform**: The support system around your software - the CLI, IDE integrations, build plugins, debugging, analytics, tools. 
 And the platform to connect and integrate Destack software together.

**Destack is designed as an integrated system**, **but you *can* pick and choose any pieces you like.**
You can just use regular TypeScript (nothing wrong with that!), pick only some of the libraries, or plug into the platform from another system entirely.

## Getting Started

 - TODO #Incomplete: getting started (`bun i destack`, `curl destack.sh/install`, ..)
 - Join the [Discord](https://discord.gg/xUFQ45TWYd)

## Higher-Order Software

We're very early in software as an industry.
Software is broken, slow, and hard to build right.
Computers are now orders of magnitude faster, yet software feels even slower.
Destack wants to make building correct, optimal, integrated software the standard.

TypeScript is a language that describes *what data looks like*.
Destack is a system for describing *how software behaves*.
Think of `.ds` as extending TypeScript for entire software systems, much like `.tsx` extends TypeScript for UI-shaped problems.

The best programing language is the one that fits the problem.
And this "language" encompasses its supporting libraries, platform and ecosystem.
The more we can express in the language, the more the toolchain can verify, optimize, and assist.
Higher-order software is raises the level of abstraction on what software systems can express *reliably*.

# Development

Destack is in very active development with a singular focus: a fully integrated software stack for optimal, correct, integrated software. 
We welcome feedback and issues, but please check in for larger changes 

## Structure

This is the open source monorepo containing the language, library, and core platform:

| Directory    | Description                                                    |
|--------------|----------------------------------------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.)    |
| `library/`   | Standard library (entity, telemetry, math, physics, UI, etc.)  |
| `platform/`  | Platform features (CLI, IDE integrations, build plugins, etc.) |
| `examples/`  | Example projects                                               |
| `templates/` | Project templates for `destack new`                            |

## Setup

Destack is *developed* using Rust and TypeScript:

- [Bun](https://bun.sh/): JavaScript runtime and package management
- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see `rust-toolchain.toml`)
- [just](https://github.com/casey/just): Scripts and command runner
- [Python 3](https://python.org/): Scripts and codegen utilities (optional)

## Development

```sh
just check          # check / lint everything
just fmt            # format all code
just lint           # lint all code
just language/test  # run only Rust tests
just library/test   # run only TS tests
```

## License

Apache-2.0. See [LICENSE.txt](LICENSE.txt).
The Destack language, toolchain, library and core platform are fully open source.
