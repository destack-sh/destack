<div align="center">
<p>
    <img src="./.github/assets/banner.svg" width="960" alt="Destack banner">
</p>

# Destack: Universal Software Engine

**Destack is a universal software engine for building correct, optimal, integrated software systems.**

Open source TypeScript(++) toolchain, VM, AOT compiler, runtime, library, services, and apps.

<p>
    <a href="VERSION"><img src="https://img.shields.io/badge/version-0.55.2-2ea44f?style=for-the-badge" alt="Version"></a>
    <a href="https://github.com/destack-sh/destack/actions/workflows/nightly.yml"><img src="https://img.shields.io/github/actions/workflow/status/destack-sh/destack/nightly.yml?branch=main&logo=github&style=for-the-badge" alt="Nightly"></a>
    <a href="LICENSE.txt"><img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="MIT License"></a>
    <a title="Discord" target="_blank" href="https://discord.gg/xUFQ45TWYd"><img alt="Chat with Destack people on Discord" src="https://img.shields.io/discord/1079840654466752606?label=Discord&logo=discord&logoColor=white&style=for-the-badge"></a>
</p>

</div>

---

## The Destack

Destack is a universal software engine with a language, runtime, libraries, services, and apps built on top of TypeScript and the open web ecosystem.
Conceptually, Destack is the antithesis to the very idea of a "stack":
instead of wrangling many disparate languages, tools, libraries, approaches, and products, Destack unifies the processes of software development into _one_ computing stack:

- [**Destack Language**](language/README.md): TypeScript(++) toolchain, VM, AOT compiler, runtime.
- [**Destack Library**](library/README.md): Standard library for most things most software needs.
- [**Destack Services**](service/README.md): Runtime, infra and developer services.
- [**Destack Apps**](app/README.md): First-party applications and developer tools.
- [**Destack Bridge**](bridge/README.md): External-facing SDKs, editor integrations, and host tooling bridges.
- [**Destack Templates**](template/README.md): Ready-to-clone starter kits for common use cases.

While Destack is designed from the ground up as an integrated system, you are of course free to pick and choose only the components you like.
It's all open source, open standards, zero lock-in - and, most importantly: highly experimental.

---

## Higher-Order Programming

It has been over 50 years since C introduced higher order programming as we know it today, yet programming is still astoundingly immature.
We routinely fail to build trivial software correctly, and even when it works, it is incredibly inefficient, and even when it is, it's not well integrated with other software.

Software is very useful, we have a lot of it, and there is about to be much, much more.
There are even new exciting possibilities to marry symbolic and probabilistic computation.
But we believe the deep opaqueness, inefficiency, and fragmentation of software can only be solved by reimagining the full software process; in the limit, that means unifying the disparate parts that have remained separate purely for historical reasons.

The more we can express in software, the higher order the abstractions we can program.
In the beginning, software was the digital shadow of "real" systems, but done correctly, software is an enabling technology for new systems that were previously impossible.
There is great promise in turning more things _into_ correct, optimal, integrated software systems, and we believe a universal software engine is the best way to enable them.

---

## Why You Shouldn't Use Destack

> [!WARNING]
> **Destack is an alpha-stage, experimental computing stack.**
> Use at your own risk. Things may change or break or disappear entirely without notice.

Destack has been in development for years and underwent a _lot_ of iteration, and it intentionally follows known good standards like TypeScript, TSX, Node and Web-shaped APIs.
However, obviously, it is still rather early, it is definitely quite radical, and there are sound arguments against the Destack-shaped "universal software engine" way:

1. **Maybe the existing stack is already good**: The existing "stack", its layers and components exist for a good reason and have withstood significant evolutionary pressure, thus trying to combine or even rearrange them in a very different way may very well turn out net negative.
2. **Maybe Destack is too Destack-special**: Destack is compatible with JS/TS, yes, and runs modern TS, yes, but many of the most interesting features only work with "modern" TS, and especially when integrating with more of the "destack" stack, which is a bigger shift.
3. **Maybe any ecosystem split is too expensive**: The ecosystem fork implied by any new language and paradigm is costly, and while transforming code is now significantly cheaper than it used to be, transforming understanding and habits and the "hard" ecosystem bits still have non-trivial friction.
4. **Maybe Destack should be more radical**: The existing (web) standards should be followed _less_ and since code transformation is now relatively cheap, and this is a unique time of disruption, Destack should be even _more_ adventorous and experimtal in its design to finally do software in the "most optimal" way.
5. **Maybe Destack should be less radical**: The existing (web) standards should be followed _more_ religiously, we shouldn't just pick and choose the "best" ones; they are pretty good by now and while they're not perfect, any deviation necessarily implies imperfect transformation at some lossy edge.
6. **Maybe "TS++" is too complex and weird**: The "++" in our "TS++" language is trying to do too much; TypeScript is not meant to be load-bearing in this way, and systems programming should be left to "true" systems languages.
7. **Maybe Destack is too complex and weird** Following TS/TSX/Node/Web standards is nice, but there is still a novel combination of features and technologies here, and the ways of working and processes required to make the most of Destack are unconventional.
8. **Maybe this really is good but it doesn't matter**: A non-trivial part of the value of the common "stack", much like with other technologies, comes from having been around for a while and thus to have stood the test of time; any new way of doing things is thus inherently suspicious, _even if_ it is "objectively" better overall.
9. ...

**If you _really_ insist on using Destack:**
- Install via `curl -fsSL https://destack.sh/install | sh` or `npm i -g @destack/cli`.
- Create a new app with `npm create destack@latest my-app` or `bun create destack my-app`.

---

## Questions You Should be Asking

...

---

## Platforms and Targets

Destack supports the web, of course, and also runs natively on Linux, macOS, and Windows as Tier 1 targets, with mobile (iOS, Android) still coming online.

| Tier | Target triples |
|------|----------------|
| **Tier 1: full support** | `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` |
| **Tier 2: pending support** | `aarch64-apple-ios`, `aarch64-linux-android` |
| **Tier 3: eventual support** | `wasm32-wasip1` |

See [TARGETS.md](TARGETS.md).

## Contributing

Destack is in [very active development](CONTRIBUTING.md) with a singular focus: a fully integrated software stack for optimal, correct, integrated software systems.
We welcome feedback, issues, ideas, and small fixes, but please reach out first for non-trivial contributions.
Large unsolicited PRs will be closed.
See [TESTING.md](TESTING.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for more.

## License

The Destack language, toolchain, library, service, app, bridge, and template are fully open source under the MIT license.
See [LICENSE.txt](LICENSE.txt).

Destack includes components licensed, vendored and integrated from third parties, which come with their own licenses including the Apache-2.0 (WITH LLVM-exception) license.
See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
