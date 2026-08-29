---
title: Conversion
description: Lossless casts and fallible conversion.
---

# Conversion

`as` accepts lossless casts. Conversions that can fail use `TryFrom` and `TryInto`.

```ds:src/conversion.ds
import { TryInto } from "destack:convert";
import { Result } from "destack:error";

const byte: uint8 = 42;
const count = byte as uint32;
const ratio = count as float64;

function port(value: string): Result<uint16> {
    value.tryInto<uint16>()
}
```
