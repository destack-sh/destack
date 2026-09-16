---
title: Test
description: Jest/Vitest style tests.
---

# Test

- jest/vitest style tests

```ds:src/add.test.ds
import { expect, test } from "destack:test";

test("adds values", () => {
    expect(20 + 22).toBe(42);
});
```

```sh
destack test
```
