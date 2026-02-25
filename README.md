<div align="center">

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

_Full-stack TypeScript(++) toolchain, VM, AOT compiler, runtime, library, and platform built on open standards._

<p>
    <img src="./.github/assets/banner.svg" width="960" alt="Destack banner">
</p>

<p>
    <a href="version.txt"><img src="https://img.shields.io/badge/version-0.55.2-2ea44f" alt="Version"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/ci.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/nightly.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/nightly.yml/badge.svg" alt="Nightly"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/release.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/release.yml/badge.svg" alt="Release"></a>
    <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
</p>
<p>
    <a title="Commits" target="_blank" href="https://github.com/destack-sh/destack/commits/main"><img src="https://img.shields.io/github/commit-activity/m/destack-sh/destack.svg?style=flat-square"></a>
    <a title="Last Commit" target="_blank" href="https://github.com/destack-sh/destack/commits/main"><img src="https://img.shields.io/github/last-commit/destack-sh/destack.svg?style=flat-square&color=FF9900"></a>
    <a title="Discord" target="_blank" href="https://discord.gg/xUFQ45TWYd"><img alt="Chat with Destack people on Discord" src="https://img.shields.io/discord/1079840654466752606?label=Discord&logo=Discord&style=social&label=Users"></a>
</p>

</div>

---

## The Destack

Destack is a fully integrated software engine with a language, runtime, libraries, and platform on top of TypeScript and the open web ecosystem.
At its essence, Destack is the antithesis to the very idea of a "stack".
Instead of wrangling many disparate tools, libraries, and approaches, Destack unifies the process of software development into one integrated system:

- [**Destack Language**](language/README.md): TypeScript(++) toolchain, VM, AOT compiler, runtime.
- [**Destack Library**](library/README.md): Standard library packages for most things most software needs, (written in TS++).
- [**Destack Client**](client/README.md): User-facing SDKs and bindings for JS/TS, WASM, Rust, and Python.
- [**Destack Platform**](platform/README.md): CLI, daemon, LSP, editor integrations, everything to run, deploy, and integrate software.
- [**Destack Examples**](examples/README.md): End-to-end sample projects that demonstrate language and platform usage.
- [**Destack Templates**](templates/README.md): Starter project templates used by `destack new`.

While Destack is designed from the ground up as an integrated system, you are of course free to pick and choose only the components you like.
It's all open source.

### Higher-Order Programming

Over 50 years since the invention of higher order programming with the introduction of the C programming language, the production and deployment of software is still astoundingly immature. 
We routinely fail to build even trivial software correctly, and even when it works, it is incredibly inefficient, and even when it is, it's not well integrated with other software.

Software is very useful, we have a lot of it, and there is about to be much, much more. 
Probabilistic computing promises new magical features, but is even harder to make robust and reliable.
We believe the opaqueness, inefficiency, and fragmentation of software as a whole can only be fully solved by rethinking the entire development process and unifying the disparate parts that have been separated purely for historical reasons.

The more we can express in one unified software system, the more great software systems we can build that understands more about what we're actually trying to do.
At its best, software is not just a pale digital shadow of a real world process, but an enabling technology to support processes that weren't possible before.
There is significant promise in turning more things _into_ correct, optimal, integrated software systems, and we believe a unified software system is the best way.

## Getting Started

> [!WARNING]
> **Destack is alpha-stage software.**
> Use at your own risk. Things may change or break without notice.

<!--TODO #Incomplete: getting started (`bun i -g @destack/cli`, `curl destack.sh/install`, and local development setup)-->

Install via `curl -fsSL https://destack.sh/install | sh` or `npm i -g @destack/cli`.
Create a new app with `npm create destack@latest my-app` or `bun create destack my-app`.

---

## Targets

Destack supports Linux, MacOS and Windows as TIer 1 targets, with mobile (iOS, Android) still coming online. 
See [TARGETS.md](TARGETS.md).

| Tier | Target triples |
|-----------|--------|
| Tier 1 | `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` |
| Tier 2 | `aarch64-apple-ios`, `aarch64-linux-android` |
| Tier 3 | `wasm32-wasip1` |

## Contributing

Destack is in [very active development](CONTRIBUTING.md) with a singular focus: a fully integrated software stack for optimal, correct, integrated software systems.
We welcome feedback, issues, ideas, and small fixes, but please reach out first for non-trivial contributions.
See [TESTING.md](TESTING.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

The Destack language, toolchain, library, and platform are fully open source under the MIT license.
See [LICENSE.txt](LICENSE.txt).

Destack includes components licensed, vendored and integrated from third parties, which come with their own licenses including the Apache-2.0 (WITH LLVM-exception) license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
