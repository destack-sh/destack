---
title: Stack
description: Shared application resources.
---

# Stack

Copy the template with the desired package name.

```ts
{ name: "@florian/stack", dependencies: {} }
```

Applications import its declarations.

```ts
import { credentials, database, files } from "@florian/stack";
```
