---
title: Fast
description: Compiled, native when you want it, and in control when it matters.
---

# Fast

TS++ compiles ahead of time, to native code, WASM, or JS, in addition to running in its own bytecode VM.
Additionally, TS++ gives you control over memory on an opt-in gradient:
 1. Structs are values: no headers, no hidden classes, copied like numbers.
 2. You can see and pick how types are laid out in memory.
 3. Ownership and borrowing are there when a GC isn't desirable (and they're optional).
 4. Workers run in parallel, on real threads (but keeping local by default ergonomics).
 5. Shared memory that is fast and sound and Rust-y (for communicating across those workers beyond `postMessage`).

## Structs

A `struct` is a plain value: it lives inline where you put it, and assigning it copies it.

```tspp
struct Point {
    x: float64;
    y: float64;
}

function moved(origin: Point): Point {
    let pivot = origin;
    pivot.x += 1.0;
    pivot
}
```

## Layout

Types have a predictable layout, and `@repr` pins it down when it matters, for example to match C or a wire format:

```tspp
@repr("C", { packed: true })
struct WireHeader {
    tag: uint8;
    size: uint32;
}

const size = sizeOf<WireHeader>();
```

## Ownership, When You Want It

Classes are managed like in TS: you share them freely and the collector cleans up.
When you want tighter control, `^T` owns a value outright and borrows lend it out, all checked at compile time:

```tspp
struct Buffer {
    size: int32;
}

function grow(buffer: &exclusive Buffer): void {
    buffer.size *= 2;
}

function measure(buffer: &readonly Buffer): int32 {
    buffer.size
}

function main(): int32 {
    let buffer: ^Buffer = Buffer { size: 8 };
    grow(&exclusive buffer);
    measure(&readonly buffer)
}
```

## Workers

Workers run in parallel on real threads, and the checker makes sure only values that are safe to share cross between them:

```tspp
import { WorkerError, spawn } from "tspp:worker";

async function double(): Promise<Result<uint64, WorkerError>> {
    const worker = spawn((value: uint64) => value * 2);
    await worker.request(21)
}
```
