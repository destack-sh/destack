<div align="center">

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

_Full-stack TypeScript(++) toolchain, VM, AOT compiler, runtime, library, and platform - built on open standards._

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

Destack is designed as an integrated full-stack system across language, runtime, libraries, and platform on top of TypeScript and the web ecosystem.
However, Destack is optimized for incremental adoption, and you are free to pick and choose any pieces you like.

- [**Destack Language**](language/README.md): TypeScript++ toolchain, VM, AOT compiler, runtime.
- [**Destack Library**](library/README.md): Standard library packages for most things most software needs, written in TS++.
- [**Destack Platform**](platform/README.md): CLI, daemon, LSP, editor integrations, everything to run, deploy, and integrate software.

### Higher-Order Programming

We're very early in the history of programming, even over 50 years .
We have a lot of software, and there is about to be much, much more.
At the same time, probabilistic software promises amazing features with built-in bugs.

Computers are miraciously fast, but software is buggy, slow, and deceptively difficult to build right.
Destack is an integrated system for describing the shape of software:
the more we can express in one unified software system, the more software systems we can build.

## Getting Started

<!--TODO #Incomplete: getting started (`bun i -g @destack-sh/cli`, `curl destack.sh/install`, and local development setup)-->

Install via `curl -fsSL https://destack.sh/install | sh` or `npm i -g @destack-sh/cli`.

---

## Status

Destack is in very active development and confidently pre-1.0, alpha-stage software.
Core language and tooling surprisingly usable for experimentation and the earliest of adopters.
APIs, CLI behavior, and project structure may change on minor releases.

## Contributing

Destack is in [very active development](CONTRIBUTING.md) with a singular focus: a fully integrated software stack for optimal, correct, integrated software systems.
We welcome feedback, issues, ideas, and small fixes, but please reach out first for non-trivial contributions.
See [TESTING.md](TESTING.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

The Destack language, toolchain, library, and platform are fully open source under the MIT license.
See [LICENSE.txt](LICENSE.txt).

Destack includes components licensed, vendored and integrated from third parties, which come with their own licenses including the Apache-2.0 (WITH LLVM-exception) license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
