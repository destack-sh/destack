---
title: Familiar
description: Most of what you know from TS just works.
---

# Familiar

TS++ starts from modern strict TS, and tries very hard to carry over as much as possible:
 1. Syntax: functions, classes, interfaces, generics, unions, template strings, _all_ the type operators.
 2. Modules: plain ESM `import` and `export`.
 3. "Standard library": `console`, `fs`, `net` and friends, much like Node and the Web standards.
 4. TSX, for when you want tree-shaped things.
 5. Packaging: `package.json`, `dependencies`, `workspaces`.

So this is TS++, and it's almost TS too:

```tspp
interface User {
    readonly id: uint32;
    name: string;
    email?: string;
}

type Role = "admin" | "member" | "guest";

function label(user: User, role: Role): string {
    const name = user.email ?? user.name;
    role == "admin" ? `${name} (admin)` : name
}
```

The "almost" is a short list, and most of it is spelling:

| TS | TS++ | Why |
| --- | --- | --- |
| `number` | `int32`, `float64`, … | Numbers have sizes, see [Predictable](30-predictable.md). |
| `T extends Bound` | `T: Bound` | Shorter, and it's not inheritance. |
| `[A, B]` | `(A, B)` | Tuples are their own thing, arrays are arrays. |
| `typeof value == "string"` | `value is string` | One way to ask for a type, and it works for every type. |
| methods override silently | `virtual` and `override` | See [Strict](40-strict.md). |
| `return` at the end | the last expression is the value | `return` still works. |
| `throw` and `try` | `Result` and `?` | Errors are values. |

## Functions

Default parameters, rest parameters, arrow functions and closures all work like in TS:

```tspp
function greet(name: string, greeting: string = "hello"): string {
    `${greeting}, ${name}`
}

function sum(...values: int32[]): int32 {
    values.reduce((total, value) => total + value, 0)
}

function adder(step: int32): (value: int32) => int32 {
    (value) => value + step
}
```

## Objects and Arrays

Object literals, spread, destructuring, and the array methods you'd expect:

```tspp
type Point = { x: float64; y: float64 };

function length({ x, y }: Point): float64 {
    Math.sqrt(x * x + y * y)
}

const origin: Point = { x: 0.0, y: 0.0 };
const moved: Point = { ...origin, x: 1.0 };

const numbers: int32[] = [3, 1, 2];
const sorted = numbers.toSorted();
const doubled = numbers.map((number) => number * 2);
const found = numbers.find((number) => number > 1);
```

Tuples are written with parentheses, and destructure like arrays:

```tspp
function swap(pair: (int32, string)): (string, int32) {
    const [number, text] = pair;
    (text, number)
}
```

## Control Flow

`if`, `for`, `for…of`, `while`, `switch`, `?.` and `??`:

```tspp
interface Config {
    server?: { port?: uint16 };
}

function port(config: Config): uint16 {
    config.server?.port ?? 8080
}

function total(prices: float64[]): float64 {
    let total = 0.0;
    for (const price of prices) {
        total += price;
    }
    total
}

function status(code: int32): string {
    switch (code) {
        case 200:
            return "ok";
        case 404:
            return "not found";
        default:
            return "other";
    }
}
```

## Narrowing

Narrowing a local works the way you're used to, with discriminants, `is`, and `instanceof`:

```tspp
type Event =
    | { kind: "ready"; port: uint16 }
    | { kind: "closed"; reason: string };

function describe(event: Event | undefined): string {
    if (event == undefined) {
        "nothing"
    } else if (event.kind == "ready") {
        `listening on ${event.port}`
    } else {
        event.reason
    }
}

function text(value: string | int32): string {
    if (value is string) {
        value
    } else {
        "a number"
    }
}
```

(Note: narrowing a field is a little stricter than in TS, since a call could change the field under you, see [Sound](20-sound.md#narrowing-stays-true).)

## Classes

Fields, constructors, `extends`, `implements`, `super`, getters, and statics:

```tspp
interface Shape {
    area(): float64;
}

class Circle implements Shape {
    radius: float64;

    constructor(radius: float64) {
        this.radius = radius;
    }

    virtual area(): float64 {
        3.14159 * this.radius * this.radius
    }
}

class Ring extends Circle {
    inner: float64;

    constructor(radius: float64, inner: float64) {
        super(radius);
        this.inner = inner;
    }

    override area(): float64 {
        super.area() - 3.14159 * this.inner * this.inner
    }
}

class Temperature {
    celsius: float64 = 0.0;

    get fahrenheit(): float64 {
        this.celsius * 1.8 + 32.0
    }

    static freezing(): Temperature {
        new Temperature()
    }
}
```

## Generics

Generic functions, classes and interfaces, with bounds written `T: Bound`:

```tspp
interface Named {
    name: string;
}

function names<T: Named>(items: T[]): string[] {
    items.map((item) => item.name)
}

class Stack<T> {
    items: T[] = [];

    push(item: T): void {
        this.items.push(item);
    }

    pop(): T | undefined {
        this.items.pop()
    }
}
```

## Enums

Numeric enums count up from `0`, and string enums carry their strings:

```tspp
enum Direction {
    Up,
    Down,
}

enum Level {
    Info = "info",
    Error = "error",
}

function flip(direction: Direction): Direction {
    direction == Direction.Up ? Direction.Down : Direction.Up
}
```

## Collections

`Map` and `Set`, the usual way:

```tspp
function count(words: string[]): Map<string, int32> {
    const counts = new Map<string, int32>();
    for (const word of words) {
        counts.set(word, (counts.get(word) ?? 0) + 1);
    }
    counts
}

function unique(words: string[]): Set<string> {
    new Set(words)
}
```

## Async

`async`, `await`, and `Promise`:

```tspp
declare function load(id: uint32): Promise<string>;

async function both(): Promise<string> {
    const first = await load(1);
    const second = await load(2);
    `${first} and ${second}`
}
```

## Modules

Modules are plain ESM, file extensions optional:

```tspp src/geometry.tspp
export function square(value: float64): float64 {
    value * value
}
```

```tspp src/main.tspp
import { square } from "./geometry";

const area: float64 = square(4.0);
```
