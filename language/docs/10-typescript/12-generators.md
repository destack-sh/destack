---
title: Generators
description: Worker-local suspended generators.
---

# Generators

```ds
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}
```

## Iteration

```ds
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}

let total: int32 = 0;
for (const value of count(3)) {
    total += value;
}
```

## Async Generators

```ds
async function* stream(): AsyncGenerator<int32, void, void> {
    yield 1;
}

async function sum(): Promise<int32> {
    let total: int32 = 0;
    for await (const value of stream()) {
        total += value;
    }

    return total;
}
```
