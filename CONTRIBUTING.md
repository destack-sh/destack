# Contributing

> **You don't need to do any of this to *use* Destack!** Go to [Getting Started](README.md#getting-started) to install and run Destack.

We welcome bug reports, fixes, and feature suggestions and any other ideas.
Join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

PRs for small fixes are probably fine, but larger unsolicited PRs are unlikely to be accepted - the whole point of the project is tight integration.
Destack is in very active development with a singular focus: a fully integrated software stack for optimal, correct, integrated software. 

## Structure

This is the open source monorepo containing the language, library, and platform core:

| Directory | Description | README |
|--------------|----------------------------------------------------------------|-------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.)    | [language/README](language/README.md)  |
| `library/`   | Standard library (entity, telemetry, math, physics, UI, etc.)  | [library/README](library/README.md)   |
| `platform/`  | Platform features (CLI, IDE integrations, build plugins, etc.) | [platform/README](platform/README.md)  |
| `examples/`  | Example projects                                               | [examples/README](examples/README.md)  |
| `templates/` | Project templates for `destack new`                            | [templates/README](templates/README.md) |


## Setup

Destack is *developed* primarily using Rust and TypeScript (and Destack itself, of course).
To contribute to Destack and build it yourself locally you will need at least `cargo`, `bun`, and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Scripts and codegen utilities (*optional*)

## Commands

We use `justfile`s as the source of truth for all commands: 
```sh
just install 		# setup everything
just check          # check & lint everything
just fmt            # format all code
just lint           # lint all code
just test           # run all tests
just bench          # run all benchmarks
just fuzz           # run all fuzzers
just clean          # clean all build artifacts
just publish        # publish all packages
```

