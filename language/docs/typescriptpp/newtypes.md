---
title: Newtypes
description: Distinct types over existing representations.
order: 202
---

# Newtypes

```ds:src/user.ds
export newtype UserId = string;

export const administrator = UserId("usr_admin");
```
