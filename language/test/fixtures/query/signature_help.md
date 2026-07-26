# Signature Help

## Function Calls

### Select the active argument

Signature help shows the selected callable and active parameter.

```ds main.ds
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, 2);
                      ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### [ignored] Select an argument after a trailing comma

A cursor gap after a comma selects the next parameter.

```ds main.ds
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, );
                      ^ cursor
```

```query signature_help main.ds#cursor
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Return a zero-parameter signature

A callable without parameters still has signature help and no active parameter.

```ds main.ds
function ping(): void {}

ping();
     ^ cursor
```

```query signature_help main.ds#cursor
@signature_help.signature index=0 label="ping(): void" active=true
```

### [ignored] Follow named argument bindings

The active parameter follows the checker binding rather than authored argument order.

```ds main.ds
function greet(name: string, greeting: string): void {}

greet(greeting: "Hello", name: "World");
                               ^^^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="greet(name: string, greeting: string): void" active=true
@signature_help.parameter signature=0 index=0 label="name: string" active=true
@signature_help.parameter signature=0 index=1 label="greeting: string"
```

### Select a rest parameter

Every argument bound into a rest parameter selects that parameter.

```ds main.ds
function sum(first: int32, ...rest: int32[]): int32 {
    return first;
}

const result = sum(1, 2, 3);
                         ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="sum(first: int32, ...rest: int32[]): int32" active=true
@signature_help.parameter signature=0 index=0 label="first: int32"
@signature_help.parameter signature=0 index=1 label="...rest: int32[]" active=true
```

## Selection

### Return the selected overload

Signature help reports only the overload selected by checking.

```ds main.ds
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse("one");
                    ^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="parse(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

### Apply selected generic arguments

Signature help displays the callable type selected for the concrete call.

```ds main.ds
function identity<Value>(value: Value): Value {
    return value;
}

declare const name: string;
const result = identity(name);
                        ^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="identity<string>(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

## Documentation

### [ignored] Include callable and parameter documentation

Signature help retains documentation from the declaration and its parameters.

```ds main.ds
/// Add two values.
/// @param left The first value.
/// @param right The second value.
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, 2);
                   ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" documentation="Add two values." active=true
@signature_help.parameter signature=0 index=0 label="left: int32" documentation="The first value." active=true
@signature_help.parameter signature=0 index=1 label="right: int32" documentation="The second value."
```

## Imported Functions

### Resolve an imported callable

An imported call uses the declaration from its defining module.

```ds math.ds
export function add(left: int32, right: int32): int32 {
    return left + right;
}
```

```ds main.ds
import { add } from "./math.ds";

const result = add(1, 2);
                   ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32" active=true
@signature_help.parameter signature=0 index=1 label="right: int32"
```

## Methods

### Resolve a method call

Method signature help uses the selected member declaration.

```ds main.ds
class Calculator {
    add(left: int32, right: int32): int32 {
        return left + right;
    }
}

function calculate(calculator: Calculator): int32 {
    return calculator.add(1, 2);
                             ^ argument
}
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Resolve an extension method call

Extension signature help uses the exact selected extension member.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
        return left + right;
    }
}

function total(calculator: Calculator): int32 {
    return calculator.add(1, 2);
                             ^ argument
}
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### [ignored] Include method parameter documentation

Method signature help includes documentation from the selected member.

```ds main.ds
class Calculator {
    /// Add two values.
    /// @param left The first value.
    /// @param right The second value.
    add(left: int32, right: int32): int32 {
        return left + right;
    }
}

function calculate(calculator: Calculator): int32 {
    return calculator.add(1, 2);
                          ^ argument
}
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" documentation="Add two values." active=true
@signature_help.parameter signature=0 index=0 label="left: int32" documentation="The first value." active=true
@signature_help.parameter signature=0 index=1 label="right: int32" documentation="The second value."
```

## Namespace Imports

### Resolve a namespace-imported callable

Namespace calls retain the exported parameter names and types.

```ds library.ds
export function paint(color: string, coats: int32): void {}
```

```ds main.ds
import * as library from "./library.ds";

library.paint("blue", 2);
                       ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="paint(color: string, coats: int32): void" active=true
@signature_help.parameter signature=0 index=0 label="color: string"
@signature_help.parameter signature=0 index=1 label="coats: int32" active=true
```

## Re-Exports

### Resolve a default re-exported callable

Signature help follows the callable declaration through a re-export alias.

```ds library.ds
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```ds barrel.ds
export { default as scale } from "./library.ds";
```

```ds main.ds
import { scale } from "./barrel.ds";

const result = scale(2, 3);
                         ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="scale(value: int32, factor: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="value: int32"
@signature_help.parameter signature=0 index=1 label="factor: int32" active=true
```

## Nested Calls

### Select the innermost call

Nested call lookup returns the innermost argument list.

```ds main.ds
function pair(left: int32, right: int32): int32 {
    return left + right;
}

function identity(value: int32): int32 {
    return value;
}

const result = identity(pair(1, 2));
                                ^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="pair(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

## Callable Values

### Resolve an expression-valued callable

A binding with a declared function type supplies its own signature help.

```ds main.ds
declare const callback: (value: string) => string;

const result = callback("ready");
                        ^^^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="callback(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

## Construction

### Resolve an explicit class constructor

Class construction reports the selected constructor parameters and nominal result.

```ds main.ds
class User {
    constructor(name: string) {}
}

const user = new User("Ada");
                      ^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="User(name: string): User" active=true
@signature_help.parameter signature=0 index=0 label="name: string" active=true
```

### Resolve a default class constructor

A class without an authored constructor still reports its zero-parameter construction signature.

```ds main.ds
class User {}

const user = new User();
                      ^ cursor
```

```query signature_help main.ds#cursor
@signature_help.signature index=0 label="User(): User" active=true
```

### Resolve a newtype constructor

Newtype construction reports its backing value parameter and nominal result.

```ds main.ds
newtype UserId = string;

const userId = UserId("user-1");
                      ^^^^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="UserId(value: string): UserId" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

### Resolve a tagged variant constructor

Tagged construction reports the selected case payload and nominal result.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;

const status = Status.Ok({ value: "ready" });
                           ^^^^^^^^^^^^^^^^ argument
```

```query signature_help main.ds#argument
@signature_help.signature index=0 label="Status.Ok({ value: string }): Status" active=true
@signature_help.parameter signature=0 index=0 label="{ value: string }" active=true
```

## Empty Results

### Return no signature for an unresolved call

An unresolved callee has no signature.

```ds main.ds
missing(1);
        ^ argument
```

```query signature_help main.ds#argument
@signature_help.none
```

### Return no signature outside call arguments

A callable reference outside an argument list has no signature-help result.

```ds main.ds
function greet(name: string): string {
    return name;
}

const callable = greet;
                 ^^^^^ reference
```

```query signature_help main.ds#reference
@signature_help.none
```
