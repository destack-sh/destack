---
title: Test
description: Jest/Vitest style tests.
---

# Test

- jest/vitest style tests

```tspp src/add.test.tspp
import { expect, test } from "tspp:test";

test("adds values", () => {
    expect(20 + 22).toBe(42);
});
```

```sh
destack test
```
