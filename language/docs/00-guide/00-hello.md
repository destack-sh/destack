---
title: Hello
description: How to run `console.log` in TS++.
---

# Hello

TS++ programs are almost exactly like TS programs.
If you've written or read any TS code, you can already read TS++:

```tspp src/main.tspp
import { log } from "tspp:console";

function greet(name: string): string {
    `hello, ${name}`
}

log(greet("world"));
```

Two small differences show up even here:
 1. The standard library imports from `tspp:`, much like `node:` in Node.
 2. The last expression of a block is its value, so `greet` needs no `return` (you can still write `return` if you want).

## A Package

A package is just a folder with a `package.json` that says TS++ is the package manager:

```json package.json
{
    "name": "hello",
    "packageManager": "tspp@2026.9.0",
    "targets": { "default": { "output": "program" } }
}
```

Everything else works like you'd expect from NPM: `exports`, `dependencies`, `workspaces`, all the usual stuff.
(Note: `packageManager` is just how the toolchain tells a TS++ package apart from any other `package.json` lying around, so NPM and friends don't bother us.)

## Check and Run

Install the toolchain, then check and run the package:

```sh
curl -fsSL https://github.com/destack-sh/tspp/releases/latest/download/install.sh | sh

tspp check
tspp run src/main.tspp
```
