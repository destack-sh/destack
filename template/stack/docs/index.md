---
title: Stack
description: Shared application resources.
---

# Stack

Copy `template/stack` into a repository and set its package name to `@<account>/stack`. Applications
depend on this package and import its declarations.

```ts
import { credentials, database, files } from "@florian/stack";
```
