
## Function Calls

### Select the active argument

Signature help shows the callable and active parameter.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, 2);
                      ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Select an argument after a trailing comma

A cursor gap after a comma selects the next parameter.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, );
                      ^ cursor
```

```query signature_help main.tspp#cursor
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Select the first parameter of an unsupplied call

A call with no written arguments selects its first parameter.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add();
                   ^ cursor
```

```query signature_help main.tspp#cursor
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32" active=true
@signature_help.parameter signature=0 index=1 label="right: int32"
```

### Return a zero-parameter signature

A callable without parameters still has signature help and no active parameter.

```tspp main.tspp
function ping(): void {}

ping();
     ^ cursor
```

```query signature_help main.tspp#cursor
@signature_help.signature index=0 label="ping(): void" active=true
```

### Select a rest parameter

Every argument bound into a rest parameter selects that parameter.

```tspp main.tspp
function sum(first: int32, ...rest: int32[]): int32 {
    return first;
}

const result = sum(1, 2, 3);
                         ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="sum(first: int32, ...rest: int32[]): int32" active=true
@signature_help.parameter signature=0 index=0 label="first: int32"
@signature_help.parameter signature=0 index=1 label="...rest: int32[]" active=true
```

### Render the current function signature

Signature help reflects the current callable declaration.

```tspp main.tspp
function format(value: string): string {
    return value;
}

const result = format("ready");
                            ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="format(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

```tspp main.tspp change
function format(value: string, radix: int32): string {
    return value;
}

const result = format("ready", 10);
                                ^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="format(value: string, radix: int32): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string"
@signature_help.parameter signature=0 index=1 label="radix: int32" active=true
```

## Selection

### Return the matching overload

Signature help reports the overload that accepts the arguments.

```tspp main.tspp
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse("one");
                    ^^^^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="parse(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

### Apply generic arguments

Signature help displays the callable type instantiated for the call.

```tspp main.tspp
function identity<Value>(value: Value): Value {
    return value;
}

declare const name: string;
const result = identity(name);
                        ^^^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="identity<string>(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

## Documentation

### Include callable and parameter documentation

Signature help includes documentation from the declaration and its parameters.

```tspp main.tspp
/// Add two values.
/// @typeParam Value - The value type.
/// @param left - The first value.
/// @param right - The second value.
/// @example
/// ```tspp
/// add(1, 2);
/// ```
function add<Value>(left: Value, right: Value): Value {
    return left;
}

const result = add(1, 2);
                   ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add<int64>(left: int64, right: int64): int64" documentation="Add two values.\n\n## Type parameters\n\n- `Value`: The value type.\n\n## Examples\n\n```tspp\nadd(1, 2);\n```" active=true
@signature_help.parameter signature=0 index=0 label="left: int64" documentation="The first value." active=true
@signature_help.parameter signature=0 index=1 label="right: int64" documentation="The second value."
```

## Imported Functions

### Resolve an imported callable

An imported call uses the declaration from its defining module.

```tspp math.tspp
export function add(left: int32, right: int32): int32 {
    return left + right;
}
```

```tspp main.tspp
import { add } from "./math.tspp";

const result = add(1, 2);
                   ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32" active=true
@signature_help.parameter signature=0 index=1 label="right: int32"
```

## Methods

### Resolve a method call

Method signature help uses the method declaration.

```tspp main.tspp
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

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Resolve an extension method call

Extension signature help uses the extension method declaration.

```tspp main.tspp
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

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

### Include method parameter documentation

Method signature help includes documentation from the method declaration.

```tspp main.tspp
class Calculator {
    /// Add two values.
    /// @param left - The first value.
    /// @param right - The second value.
    add(left: int32, right: int32): int32 {
        return left + right;
    }
}

function calculate(calculator: Calculator): int32 {
    return calculator.add(1, 2);
                          ^ argument
}
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="add(left: int32, right: int32): int32" documentation="Add two values." active=true
@signature_help.parameter signature=0 index=0 label="left: int32" documentation="The first value." active=true
@signature_help.parameter signature=0 index=1 label="right: int32" documentation="The second value."
```

## Namespace Imports

### Resolve a namespace-imported callable

Namespace calls use the exported parameter names and types.

```tspp library.tspp
export function paint(color: string, coats: int32): void {}
```

```tspp main.tspp
import * as library from "./library.tspp";

library.paint("blue", 2);
                       ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="paint(color: string, coats: int32): void" active=true
@signature_help.parameter signature=0 index=0 label="color: string"
@signature_help.parameter signature=0 index=1 label="coats: int32" active=true
```

## Re-Exports

### Resolve a default re-exported callable

Signature help follows the callable declaration through a re-export alias.

```tspp library.tspp
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```tspp barrel.tspp
export { default as scale } from "./library.tspp";
```

```tspp main.tspp
import { scale } from "./barrel.tspp";

const result = scale(2, 3);
                         ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="scale(value: int32, factor: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="value: int32"
@signature_help.parameter signature=0 index=1 label="factor: int32" active=true
```

## Nested Calls

### Select the innermost call

Nested call lookup returns the innermost argument list.

```tspp main.tspp
function pair(left: int32, right: int32): int32 {
    return left + right;
}

function identity(value: int32): int32 {
    return value;
}

const result = identity(pair(1, 2));
                                ^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="pair(left: int32, right: int32): int32" active=true
@signature_help.parameter signature=0 index=0 label="left: int32"
@signature_help.parameter signature=0 index=1 label="right: int32" active=true
```

## Callable Values

### Resolve an expression-valued callable

A binding with a declared function type supplies its own signature help.

```tspp main.tspp
declare const callback: (value: string) => string;

const result = callback("ready");
                        ^^^^^^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="callback(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

### Resolve a callable parameter

A call through a function parameter shows its function type.

```tspp main.tspp
function apply(callback: (value: string) => string): string {
    return callback("ready");
                     ^ cursor
}
```

```query signature_help main.tspp#cursor
@signature_help.signature index=0 label="callback(value: string): string" active=true
@signature_help.parameter signature=0 index=0 label="value: string" active=true
```

## Construction

### Resolve an explicit class constructor

Class construction reports its constructor parameters and nominal result.

```tspp main.tspp
class User {
    constructor(name: string) {}
}

const user = new User("Ada");
                      ^^^^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="User(name: string): User" active=true
@signature_help.parameter signature=0 index=0 label="name: string" active=true
```

### Resolve a default class constructor

A class without a constructor declaration reports its zero-parameter construction signature.

```tspp main.tspp
class User {}

const user = new User();
                      ^ cursor
```

```query signature_help main.tspp#cursor
@signature_help.signature index=0 label="User(): User" active=true
```

### Resolve a newtype constructor

Newtype construction reports its backing argument and nominal result.

```tspp main.tspp
newtype UserId = string;

const userId = UserId("user-1");
                      ^^^^^^^^ argument
```

```query signature_help main.tspp#argument
@signature_help.signature index=0 label="UserId(string): UserId" active=true
@signature_help.parameter signature=0 index=0 label=string active=true
```

## Empty Results

### Return no signature for an unresolved call

An unresolved callee has no signature.

```tspp main.tspp
missing(1);
        ^ argument
```

```query signature_help main.tspp#argument
@signature_help.none
```

### Return no signature outside call arguments

A callable reference outside an argument list has no signature-help result.

```tspp main.tspp
function greet(name: string): string {
    return name;
}

const callable = greet;
                 ^^^^^ reference
```

```query signature_help main.tspp#reference
@signature_help.none
```
