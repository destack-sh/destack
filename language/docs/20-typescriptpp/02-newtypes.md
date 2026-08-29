---
title: Newtypes
description: Distinct types over existing representations.
---

# Newtypes

```ds:src/user.ds
export newtype UserId = string;

export const administrator = UserId("usr_admin");
```
