# Rename

## Local Variables

### Rename local variable

Rename should update all occurrences of a symbol to the new name.

```ds
const foo = 1;
//    ^^^ target

const bar = foo + foo;
console.log(foo);
```

`foo` appears 4 times: its definition and 3 uses. Renaming to `baz` should update all 4 occurrences.

```query rename target "baz"
```

```expected:main
const baz = 1;

const bar = baz + baz;
console.log(baz);
```

## Classes

### Rename class with type references

Renaming a class should update its definition and all type references.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Meetup = new Meetup();
//       ^^^^^^ ref1
//                   ^^^^^^ ref2
```

Renaming `Meetup` to `Event` should only update `Meetup`, not `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Event = new Event();
```

### Rename class with field type references

Renaming a class used as a field type should work correctly.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Meetup,
//          ^^^^^^ ref
    /// The language preference.
    language: MeetupLanguage,
}
```

Renaming `Meetup` to `Event` should only update `Meetup`, not affect `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Event,
    /// The language preference.
    language: MeetupLanguage,
}
```

## Members

### Rename class method

Renaming a class method should update the definition and all call sites.

```ds
class Counter {
    inc(): int32 {
//  ^^^ target
        return 1;
    }
}

function main() {
    const counter = new Counter();
    const first = counter.inc();
    const second = counter.inc();
}
```

Renaming `inc` to `next` should update all method calls.

```query rename target "next"
```

```expected:main
class Counter {
    next(): int32 {
        return 1;
    }
}

function main() {
    const counter = new Counter();
    const first = counter.next();
    const second = counter.next();
}
```

### Rename struct field

Renaming a struct field should update the definition and field accesses.

```ds
struct Point {
    x: int32,
//  ^ target
    y: int32,
}

function length(p: Point): int32 {
    return p.x + p.x;
}
```

Renaming `x` to `z` should update all field accesses.

```query rename target "z"
```

```expected:main
struct Point {
    z: int32,
    y: int32,
}

function length(p: Point): int32 {
    return p.z + p.z;
}
```

## Scope and Shadowing

### Rename inner binding without touching outer binding

Renaming a shadowed binding should only affect the selected symbol and its references.

```ds
const value = 1;
//    ^^^^^ def:outer

function main(): int32 {
    const value = 2;
//        ^^^^^ target:inner

    return value + 1;
//         ^^^^^ use:inner
}

const total = value + 3;
//            ^^^^^ use:outer
```

Renaming the inner `value` to `innerValue` should not change the outer `value`.

```query rename target:inner "innerValue"
```

```expected:main
const value = 1;

function main(): int32 {
    const innerValue = 2;

    return innerValue + 1;
}

const total = value + 3;
```

## Import Shapes

### Rename type alias used via type-only import

Renaming a type alias should update type-only imports and type usages.

```ds:types.ds
export type Options = {
//          ^^^^^^^ target:type
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
    console.log(options.name);
}
```

Renaming `Options` to `Config` should update the export, import, and usage.

```query rename target:type "Config"
```

```expected:types
export type Config = {
    name: string,
};
```

```expected:main
import type { Config } from "./types.ds";

function configure(options: Config): void {
    console.log(options.name);
}
```

### Rename exported symbol imported with alias

Renaming an exported symbol should update the imported name but keep the local alias.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ target:alias
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet as localGreet } from "./lib.ds";

const message = localGreet("Destack");
```

Renaming `greet` to `welcome` should only change the exported and imported names.

```query rename target:alias "welcome"
```

```expected:lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import { welcome as localGreet } from "./lib.ds";

const message = localGreet("Destack");
```

### Rename exported symbol accessed via namespace import

Renaming an exported symbol should update namespace member accesses.

```ds:api.ds
export function greet(name: string): string {
//              ^^^^^ target:ns
    return "Hello, " + name;
}
```

```ds:main.ds
import * as api from "./api.ds";

const message = api.greet("Destack");
```

Renaming `greet` to `welcome` should update the namespace member access.

```query rename target:ns "welcome"
```

```expected:api
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import * as api from "./api.ds";

const message = api.welcome("Destack");
```
