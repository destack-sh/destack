# Destack

Destack is a universal software engine for building correct, optimal, integrated full-stack systems.
Build with TypeScript and the web ecosystem in one unified open source toolchain, stack and platform.
Own your software and run it anywhere, free of vendor lock-in with one integrated stack:

 - **Destack Language**: TypeScript extended for correctness, ergonomics, and performance (`.ds`, like `.tsx`; Valid TypeScript is valid Destack).
See [language/DESIGN.md](language/DESIGN.md) for language design and [language/SPECIFICATION.md](language/SPECIFICATION.md) for specifics.

 - **Destack Library**: Standard unified modules for everything software applications need across the stack entities, telemetry, math, physics, UI, and more.
Batteries included, but not required.

 - **Platform**: The support software for your software - the CLI, IDE integrations, build plugins, debugging, analytics, tools. And the platform to connect and integrate Destack software, all working together.

## Higher-Order Software

We're very early in software as an industry.
We could do so much more—software is embarrassingly broken and slow.
Building correct, optimal, integrated software should be simple and fast.

TypeScript is a language that describes *what data looks like*.
Destack is a system for describing *how software behaves* - all on top of TypeScript.
Think of it as TypeScript for entire systems (similar to how `.tsx` works).

The more we can express in the language, the more the toolchain can verify, optimize, and assist.
The best programing language is the one that fits your domain, and the whole Destack system is designed to enable *domain languages*.
That's what we mean by "higher-order software": raising the level of abstraction on what software systems can *reliably* express without losing the details.

## Structure

This is the fully open source monorepo containing the language, library, and core platform:

| Directory    | Description                                                    |
|--------------|----------------------------------------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.)    |
| `library/`   | Standard library (entity, telemetry, math, physics, UI, etc.)  |
| `platform/`  | Platform features (CLI, IDE integrations, build plugins, etc.) |
| `examples/`  | Example projects                                               |
| `templates/` | Project templates for `destack new`                            |

## Prerequisites

Destack is *developed* using Rust and TypeScript:

- [Bun](https://bun.sh/): JavaScript runtime and package management
- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see `rust-toolchain.toml`)
- [just](https://github.com/casey/just): Scripts and command runner
- [Python 3](https://python.org/): Scripts and codegen utilities (optional)

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
