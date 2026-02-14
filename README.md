<div align="center">

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

Full-stack TypeScript(++) toolchain, VM, AOT compiler, runtime, library, and platform, all built on open standards.

<p>
  <a href="https://github.com/destack-sh/destack/actions/workflows/ci.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/destack-sh/destack/actions/workflows/nightly.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/nightly.yml/badge.svg" alt="Nightly"></a>
  <a href="https://github.com/destack-sh/destack/actions/workflows/release.yml"><img src="https://github.com/destack-sh/destack/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/language-alpha-orange" alt="Language Alpha">
  <img src="https://img.shields.io/badge/library-pre--alpha-red" alt="Library Pre-Alpha">
  <img src="https://img.shields.io/badge/platform-pre--alpha-red" alt="Platform Pre-Alpha">
</p>

</div>

## The Destack

Destack is designed as an integrated full-stack system across language, runtime, libraries, and platform on top of TypeScript and the web ecosystem.
However, Destack is optimized for incremental adoption, and you are free to pick and choose any pieces you like.

| Component | Area | Status | Description | README |
|-----------|------|--------|-------------|--------|
| Destack Language | `language/` | Alpha | TypeScript++ with opt-in features for correctness, collaboration, ergonomics, and performance, with incremental `.ds` / `.d.ds` adoption. | [language/README.md](language/README.md) |
| Destack Library | `library/` | Pre-Alpha | Standard library packages and runtime bindings for integrated full-stack development. | [library/README.md](library/README.md) |
| Destack Platform | `platform/` | Pre-Alpha | CLI, daemon, LSP, editor integrations, and build tooling to connect software to real workflows. | [platform/README.md](platform/README.md) |

## Higher-Order Software, Higher-Order Development

We're very early in software as an industry.
Software is broken, suboptimal, and surprisingly hard to build right.
Computers are miraculously fast, yet software still feels slow and clunky.
We can do better.
Destack aims to make building correct, optimal, integrated software the obvious default.

TypeScript is a language for describing the shape of data.
Destack is a system for describing the shape of software.
TypeScript has `.ts`, `.d.ts`, and `.tsx`, and Destack adds `.ds` and `.d.ds`.
The integrated library and platform extend the TypeScript philosophy to whole software systems.

The best programming language is the one that fits the problem.
And this language also includes supporting libraries, tooling, and ecosystem.
The more we can express in one unified system, the more the toolchain can verify, optimize, and assist.
Higher-order software raises the level of abstraction of what software can express reliably.

## Getting Started

<!--NOTE #Incomplete: getting started (`bun i destack`, `curl destack.sh/install`, and local development setup)-->
not yet

## Contributing

Destack is in [very active development](CONTRIBUTING.md) with a singular focus: a fully integrated software stack for optimal, correct, integrated software.
We welcome feedback, issues, ideas, and small fixes, but please reach out first for non-trivial contributions.
See [TESTING.md](TESTING.md) for the test matrix and [CONTRIBUTING.md](CONTRIBUTING.md) for contribution flow.

## License

Destack is MIT licensed.
See [LICENSE.txt](LICENSE.txt) for details.
The Destack language, toolchain, library, and platform core are fully open source.

This project includes vendored components under the Apache-2.0 WITH LLVM-exception license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) for details.
