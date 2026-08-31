---
title: Ownership
description: Managed, owned, borrowed, or raw.
---

# Ownership

```ds:src/ownership.ds
class User {}

const managed: User = new User();
const owned: ^User = new User();
const borrowed: &User = &managed;
const raw: *User = *borrowed;
```
