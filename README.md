<div align="center">
<p>
    <img src="./platform/brand/banner/banner.svg" width="960" alt="Destack banner">
</p>

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

Open source cross-platform TypeScript(++) toolchain, VM, AOT compiler, runtime, library, services, and apps.

<p>
    <a href="VERSION.txt"><img src="https://img.shields.io/badge/version-0.55.4-2ea44f?style=for-the-badge" alt="Version"></a>
    <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="MIT License"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/nightly.yml"><img src="https://img.shields.io/github/actions/workflow/status/destack-sh/destack/nightly.yml?branch=main&label=Nightly&logo=github&style=for-the-badge" alt="Nightly"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/destack-sh/destack/release.yml?branch=main&label=Release&logo=github&style=for-the-badge" alt="Release"></a>
    <a title="Discord" target="_blank" href="https://discord.gg/xUFQ45TWYd"><img alt="Chat with Destack people on Discord" src="https://img.shields.io/discord/1079840654466752606?label=Discord&logo=discord&logoColor=white&style=for-the-badge"></a>
</p>

</div>

---

## The Destack

**Destack is a universal software engine with a language, compiler, toolchain, runtime, libraries, services, and apps built on top of TypeScript and the open web ecosystem.**
Mechanically, Destack is a fully integrated stack for building eventually all software systems extremely well, but conceptually, Destack is the antithesis to the very idea of a "stack":
instead of wrangling many disparate languages, tools, libraries, approaches, runtimes, services, and apps, Destack unifies the processes of software production into _one_ universal computing stack:

- [**Destack Language**](language/README.md): TypeScript(++) toolchain, VM, AOT compiler, runtime.
- [**Destack Library**](library/README.md): Rich standard library for most things most software needs.
- [**Destack Services**](service/README.md): First-party services for most things most software needs.
- [**Destack Apps**](app/README.md): First-party applications and programmer tools.
- [**Destack Templates**](template/README.md): Ready-to-clone starter kits for common use cases.
- [**Destack Bridge**](bridge/README.md): Two-way bridges for integrating the Destack universe.
- [**Destack Platform**](platform/README.md): First-party site, apps, and services (hosting the above).

The whole point of Destack is to make software systems - including itself - fully [homoiconic](https://en.wikipedia.org/wiki/Homoiconicity) and hackable with [incrementally granular](https://caseymuratori.com/blog_0016) building blocks.
The architecture is modeled around "do-it-yourself software" over "ready-to-wear software", providing a sort of meta-stack for developing correct, optimal, integrated software stacks.
We do provide _some_ common [apps](app/README.md) and [templates](template/README.md), but Destack is optimized for programmers of all kinds building their _own_ software processes in one integrated system.

## Usage

> [!WARNING]
> **Destack is an alpha-stage, _experimental_ computing stack.**
> Things may change or break or vanish.

**Get started with Destack**:
- Install Destack via `curl -fsSL https://destack.sh/install | sh`.
- Create a new Destack app with `destack new my-destack-app`.

## Rationale

Over 50 years after [C introduced higher order programming](https://en.wikipedia.org/wiki/C_(programming_language)#History) programming is still astoundingly immature.
Software "engineering" is _still_ anything but, and while our tools have gotten prettier, the fundamental motions are unchanged: text in, text out, no [_real_ confidence](https://apple.github.io/foundationdb/flow.html).
We have increasingly grown accustomed to the acrued sediment of software being buggy, slow and fragmented, but it doesn't have to be this way. 
See [Rationale](RATIONALE.md).

## Contributing

Destack is in [very active development](https://github.com/destack-sh/destack/commits/main/) with a singular focus: a fully integrated computing stack for building optimal, correct, integrated software systems.
We welcome feedback, issues, ideas, and _maybe_ some small fixes, but please [reach out](https://discord.gg/xUFQ45TWYd) first for non-trivial contributions.
See [TESTING.md](TESTING.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

Destack is fully open source under the MIT license across the language, library, services, apps, bridges, and platform.
See [LICENSE.txt](LICENSE.txt).

Destack also includes components licensed, vendored, and integrated from third parties, which come with their own licenses including but not limited to the Apache-2.0 (with LLVM-exception) license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
