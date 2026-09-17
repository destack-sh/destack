---
title: Control Flow
description: Control Flow
---

# Control Flow

- `if let` and `while let` bind a refutable pattern for the successful branch or iteration
- `loop` is the explicit infinite-loop form and produces values through `break value`
- labeled loops accept `break label: value`

## Loops

For convenience and clarity, TypeScript++ supports `loop` as the explicit infinite loop form, and like other expressions, loops can produce a value through `break`.

```ds
declare function readInput(): string;
declare function process(input: string): void;

const line = loop {
    const input = readInput();
    if (input == "quit") {
        break "done";
    }
    process(input);
};

let status = outer: loop {
    break outer: "done";
};
status satisfies "done";
```
