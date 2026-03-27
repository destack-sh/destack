# Inlay Hints

## Function Calls

### Parameter hints for function calls

Inlay hints should show parameter names at each argument position in a function call.

```ds
function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}

const msg = greet("World", "Hello");
```

`greet("World", "Hello")` has 2 arguments, so we expect 2 parameter hints plus 1 type hint for `msg`.

```query inlay_hints $0
main.ds:5:10 kind=type label=: string
main.ds:5:19 kind=parameter label=name:
main.ds:5:28 kind=parameter label=greeting:
```

### No hints for empty calls

Calls without arguments should not produce parameter hints.

```ds
function ping(): void {}
ping();
```

No hints expected for a zero argument call.

```query inlay_hints $0
<none>
```

### Parameter hints for multi-argument calls

Inlay hints count should match the number of arguments passed to a function.

```ds
function calculate(a: int32, b: int32, c: int32): int32 {
    return a + b + c;
}

const result = calculate(1, 2, 3);
```

`calculate(1, 2, 3)` has 3 arguments, so we expect 3 parameter hints plus 1 type hint for `result`.

```query inlay_hints $0
main.ds:5:13 kind=type label=: int32
main.ds:5:26 kind=parameter label=a:
main.ds:5:29 kind=parameter label=b:
main.ds:5:32 kind=parameter label=c:
```

### Parameter hints for method calls

Method calls should include parameter hints, and inferred types should be shown for results.

```ds
class Greeter {
    greet(name: string): string {
        return name;
    }
}

const greeter: Greeter = new Greeter();
const message = greeter.greet("World");
```

Expect a type hint for `message` and a parameter hint for `greet`.

```query inlay_hints $0
main.ds:8:14 kind=type label=: string
main.ds:8:31 kind=parameter label=name:
```

### Parameter hints for imported calls

Parameter hints should use DIR-resolved parameter names for imported symbols.

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```ds:main.ds
import { paint } from "./lib.ds";

const label = "red";
paint("blue", 2);
paint(label, 3);
```

Expect a type hint for `label`, plus parameter hints for literal arguments resolved from the import.

```query inlay_hints $0
main.ds:3:12 kind=type label=: string
main.ds:4:7 kind=parameter label=color:
main.ds:4:15 kind=parameter label=coats:
main.ds:5:14 kind=parameter label=coats:
```

### Keep inlay hints after malformed call statements

Inlay hints should still resolve later valid calls after one malformed call statement.

```ds
broken(,

function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2);
```

```query inlay_hints $0
main.ds:7:13 kind=type label=: int32
main.ds:7:20 kind=parameter label=x:
main.ds:7:23 kind=parameter label=y:
```

### Keep inlay hints after bare new recovery statements

Inlay hints should still resolve later valid calls after one bare `new` recovery statement.

```ds
new

function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2);
```

```query inlay_hints $0
main.ds:7:13 kind=type label=: int32
main.ds:7:20 kind=parameter label=x:
main.ds:7:23 kind=parameter label=y:
```

### Keep inlay hints after throw recovery statements

Inlay hints should still resolve later valid calls after one recovered `throw` statement.

```ds
throw

function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2);
```

```query inlay_hints $0
main.ds:7:13 kind=type label=: int32
main.ds:7:20 kind=parameter label=x:
main.ds:7:23 kind=parameter label=y:
```

### Keep inlay hints after yield star recovery statements

Inlay hints should still resolve later valid calls after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2);
```

```query inlay_hints $0
main.ds:3:16 kind=type label=: int32
main.ds:10:13 kind=type label=: int32
main.ds:10:20 kind=parameter label=x:
main.ds:10:23 kind=parameter label=y:
```

### Skip hints that repeat argument names

Parameter hints should not repeat obvious argument names.

```ds
function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}

const name = "World";
const msg = greet(name, "Hello");
```

Expect type hints for `name` and `msg`, plus a parameter hint only for `greeting`.

```query inlay_hints $0
main.ds:5:11 kind=type label=: string
main.ds:6:10 kind=type label=: string
main.ds:6:25 kind=parameter label=greeting:
```

### Skip non literal arguments in literals mode

Parameter hints should only appear for literal arguments in literals mode.

```ds
function greet(name: string): void {}

const who = "World";
greet(who);
```

Expect only a type hint for `who` and no parameter hints.

```query inlay_hints $0
main.ds:3:10 kind=type label=: string
```

### Skip hints for named arguments

Named arguments already include the parameter name, so inlay hints should be suppressed.

```ds
function greet(name: string, greeting: string): void {}

greet(name: "World", greeting: "Hello");
```

```query inlay_hints $0
<none>
```

### Treat template literals as literals

Template string literals should count as literal arguments for parameter hints.

```ds
function greet(name: string): void {}

greet(`World`);
```

```query inlay_hints $0
main.ds:3:7 kind=parameter label=name:
```

### No type hints when an explicit type annotation is present

Bindings with explicit type annotations should not receive inlay type hints.

```ds
function greet(name: string): string {
    return name;
}

const msg: string = greet("World");
```

```query inlay_hints $0
main.ds:5:27 kind=parameter label=name:
```

### No type hints for destructured bindings

Destructured binding patterns should not receive inlay type hints.

```ds
struct Point {
    x: int32
    y: int32
}

const { x, y } = Point { x: 1, y: 2 };
```

```query inlay_hints $0
<none>
```

### Skip hints for spread arguments

Spread arguments should not receive parameter name hints.

```ds
function greet(name: string): void {}

const names: string[] = ["World"];
greet(...names);
```

```query inlay_hints $0
<none>
```

### Type hints for inferred literal bindings

Bindings without explicit annotations should receive type hints.

```ds
const count = 1;
```

```query inlay_hints $0
main.ds:1:12 kind=type label=: int32
```

### Skip hints for member access arguments

Parameter hints should not appear for member access expressions.

```ds
struct Config {
    timeout: int32
}

function useTimeout(timeout: int32): void {}

const cfg = Config { timeout: 5 };
useTimeout(cfg.timeout);
```

Expect a type hint for `cfg` and no parameter hint for the member access.

```query inlay_hints $0
main.ds:7:10 kind=type label=: Config
```

### Include single-argument hints even when name is in function name

Single-argument calls should still show hints when the argument is a literal.

```ds
function setColor(color: string): void {}

setColor("red");
```

```query inlay_hints $0
main.ds:3:10 kind=parameter label=color:
```

### Include boolean literal hints

Boolean literal arguments should include parameter hints in literals mode.

```ds
function setEnabled(enabled: bool): void {}

setEnabled(true);
```

```query inlay_hints $0
main.ds:3:12 kind=parameter label=enabled:
```

### Include template literal hints

Template literal arguments should include parameter hints in literals mode.

```ds
function logMessage(message: string): void {}

logMessage(`hello`);
```

```query inlay_hints $0
main.ds:3:12 kind=parameter label=message:
```

### Skip parameter hints for spread arguments

Spread arguments should not produce parameter name hints.

```ds
function sum(a: int32, b: int32, c: int32): int32 {
    return a + b + c;
}

const args: int32[] = [1, 2, 3];
const count = 1;
sum(...args);
```

Expect a type hint for `count` and no parameter hints for the spread call.

```query inlay_hints $0
main.ds:6:12 kind=type label=: int32
```

## Type Hints

### Type hint for variable with inferred type

Type hints should show inferred types for variables without explicit annotations.

```ds
const x = 42;
const y = "hello";
```

Variables `x` and `y` should get type hints. With 0 function calls, only type hints.

```query inlay_hints $0
main.ds:1:8 kind=type label=: int32
main.ds:2:8 kind=type label=: string
```

### Type hint for inferred class instantiation

Type hints should show class types inferred from `new` expressions.

```ds
class Dog {}

const pet = new Dog();
```

```query inlay_hints $0
main.ds:3:10 kind=type label=: Dog
```

### Type hints for multiple bindings

Type hints should cover multiple bindings in the same file.

```ds
const alpha = 1;
const beta = "hi";
```

```query inlay_hints $0
main.ds:1:12 kind=type label=: int32
main.ds:2:11 kind=type label=: string
```

### Type hint for variable with explicit type

Variables with explicit type annotations should not get type hints.

```ds
const x: int32 = 42;
const y: string = "hello";
```

No type hints expected (types already annotated), no function calls.

```query inlay_hints $0
<none>
```

### Mixed parameter and type hints

Both parameter hints and type hints should be shown together.

```ds
function double(n: int32): int32 {
    return n * 2;
}

const result = double(21);
```

1 parameter hint for `double(21)`, plus 1 type hint for `result`.

```query inlay_hints $0
main.ds:5:13 kind=type label=: int32
main.ds:5:23 kind=parameter label=n:
```

## Imports

### Parameter hints for namespace imported calls

Namespace imported calls should still use the resolved parameter names.

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```ds:main.ds
import * as api from "./lib.ds";

api.paint("blue", 2);
```

```query inlay_hints $0
main.ds:3:11 kind=parameter label=color:
main.ds:3:19 kind=parameter label=coats:
```

### Parameter hints for default imported calls

Default imported calls should still use the resolved parameter names.

```ds:lib.ds
export default function repeat(text: string, times: int32): string {
    return text;
}
```

```ds:main.ds
import repeat from "./lib.ds";

repeat("hi", 2);
```

```query inlay_hints $0
main.ds:3:8 kind=parameter label=text:
main.ds:3:14 kind=parameter label=times:
```

### Parameter hints through default re-export chains

Parameter hints should still resolve through default re-export alias chains.

```ds:lib.ds
export default function format(value: int32, scale: int32): int32 {
    return value * scale;
}
```

```ds:barrel.ds
export { default as format } from "./lib.ds";
```

```ds:main.ds
import { format } from "./barrel.ds";

format(1, 2);
```

```query inlay_hints $0
main.ds:3:8 kind=parameter label=value:
main.ds:3:11 kind=parameter label=scale:
```

### Parameter hints through namespace re-export chains

Parameter hints should still resolve through namespace re-export alias chains.

```ds:lib.ds
export function paint(color: string, coats: int32): void {}
```

```ds:barrel.ds
export * as api from "./lib.ds";
```

```ds:main.ds
import { api } from "./barrel.ds";

api.paint("blue", 2);
```

```query inlay_hints $0
main.ds:3:11 kind=parameter label=color:
main.ds:3:19 kind=parameter label=coats:
```
