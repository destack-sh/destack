---
title: Setup
description: Your first program.
---

# Setup

[Install Destack](/docs/setup/), then create a project:

```sh
destack init hello
cd hello
```

```ds:src/index.ds
import { log } from "destack:console";

log("Hello, Destack!");
```

```sh
destack check src/index.ds
```

## Run Locally

```sh
destack run ./src/index.ds
```
