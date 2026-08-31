---
title: Borrowing
description: As soon as we pass and store references, we need to make sure those are safe too.
---

# Borrowing

```ds:src/borrowing.ds
function write(storage: &exclusive uint8[], index: isize, value: uint8): void {
    storage[index] = value;
}
```
