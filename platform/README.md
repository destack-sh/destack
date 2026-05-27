# Platform

Destack's own operated platform surfaces, deployment assets, and hosted stack.

## Projects

| Project | Summary |
|---------|---------|
| [`stack`](stack/README.md) | Single entrypoint for defining and deploying the operated Destack platform |
| [`site`](site) | Public web surfaces for Destack such as the website and blog |

## Commands

Run these commands from the repository root.

```sh
just platform/build
just platform/check-quick
just platform/check-full
just platform/diff
just platform/deploy
```
