# Destack

Destack is a "universal software engine", a system for building fantastic full-stack systems and applications.
Build with TypeScript and the web ecosystem in one unified open source toolchain, stack and platform.
Deploy and run anywhere with the existing ecosystem as you please, free of vendor lock-in.

Destack works with the tools you like and embodies existing best practices and conventions.
Write plain TypeScript or our Destack dialect (`.ds`, much like `.tsx` or even "TypeScript++").
Everything is absurdly integrated to enable rapid end-to-end development of *fantastic* software.

## Structure

This is the fully open source monorepo of Destack containing the language, library, and core platform tools:

| Directory    | Description                                             |
|--------------|--------------------------------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.) |
| `library/`   | Standard library (entity, telemetry) |
| `platform/`  | Platform features (CLI, IDE integrations, build plugins, etc.) |
| `examples/`  | Example projects |
| `templates/` | Project templates for `destack new` |

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
