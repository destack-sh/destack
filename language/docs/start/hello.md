---
title: Hello, Destack
description: Install Destack and build a program.
order: 21
---

# Hello, Destack

## Install

```sh
curl -fsSL https://destack.sh/install | sh
destack init hello --template app
cd hello
```

## Program

```ds:src/main.ds
import { log } from "destack:console";

export function main(): void {
    log("Hello, Destack!");
}
```

## Build

```sh
destack check
destack build
```
