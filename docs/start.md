---
title: Get started
description: Install Destack and build a small TypeScript++ application.
order: 10
---

# Get started

Install Destack, create an application, and send it through the integrated toolchain.

## Install

```sh
curl -fsSL https://destack.sh/install | sh
```

## Create

Initialize an application in a new directory:

```sh
destack init hello --template app
cd hello
```

The application entry point is ordinary TypeScript-shaped source:

```ds:src/main.ds
function main() {
    console.log("Hello, Destack!");
}

main();
```

## Check

Run the type checker, linter, and static analysis together:

```sh
destack check
```

## Build

Compile every configured target:

```sh
destack build
```

Continue with the [language guide](/docs/language/), or inspect the complete CLI with `destack --help`.
