---
title: Setup
description: Safe, sound, predictable, and above all familiar.
---

# Setup

```sh
curl -fsSL https://destack.sh/install | sh
destack init hello
cd hello
```

```ds:src/main.ds
import { log } from "destack:console";

log("Hello, Destack!");
```

```sh
destack run src/main.ds
```
