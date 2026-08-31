---
title: Results
description: Errors as values.
---

# Results

```ds:src/result.ds
function parsePort(value: string): Result<uint16, ParseError> {
    const port = uint16.parse(value)?;
    return Ok(port);
}
```
