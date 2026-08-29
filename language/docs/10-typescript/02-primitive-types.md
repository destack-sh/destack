---
title: Primitive Types
description: Booleans, numbers, characters, and strings.
---

# Primitive Types

Destack keeps TypeScript's primitive types and adds integer widths, float widths, and characters.

```ds:src/primitives.ds
const enabled: boolean = true;
const requests: uint32 = 42;
const ratio: float64 = 0.75;
const initial: char = 'D';
const name: string = "Destack";

const position: [int32, int32] = [12, 8];
const labels: string[] = ["web", "native"];
```
