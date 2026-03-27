# Call Hierarchy

## Incoming Calls

### Incoming calls to a function

Incoming calls should list the functions that call the target.

```ds
function callee(): void {
//       ^^^^^^ def:callee
}

function caller(): void {
//       ^^^^^^ def:caller
    callee();
//  ^^^^^^ use:callee
}
```

```query call_hierarchy use:callee incoming
main.ds:4:1-6:2 name=caller kind=function selection=main.ds:4:10-4:16 calls=main.ds:5:5-5:13
```

### Incoming calls from multiple functions

Incoming calls should include every function that invokes the target.

```ds
function target(): void {
//       ^^^^^^ def:target
}

function caller_one(): void {
    target();
    target();
}

function caller_two(): void {
    target();
}
```

```query call_hierarchy def:target incoming
caller_one
caller_two
```

### Incoming call ranges are captured

Incoming calls should report all call ranges within a caller.

```ds
function target(): void {
//       ^^^^^^ def:target
}

function caller(): void {
//       ^^^^^^ def:caller
    target();
    target();
}
```

```query call_hierarchy def:target incoming
main.ds:4:1-7:2 name=caller kind=function selection=main.ds:4:10-4:16 calls=main.ds:5:5-5:13|main.ds:6:5-6:13
```

## Outgoing Calls

### Outgoing calls from a function

Outgoing calls should list the functions that the target calls.

```ds
function leaf(): void {
//       ^^^^ def:leaf
}

function root(): void {
//       ^^^^ def:root
    leaf();
//  ^^^^ use:leaf
}
```

```query call_hierarchy def:root outgoing
main.ds:1:1-2:2 name=leaf kind=function selection=main.ds:1:10-1:14 calls=main.ds:5:5-5:11
```

### Outgoing calls to multiple functions

Outgoing calls should include every function invoked by the target.

```ds
function left(): void {}
function right(): void {}

function root(): void {
//       ^^^^ def:root
    left();
    right();
}
```

```query call_hierarchy def:root outgoing
left
right
```

### Outgoing call ranges are captured

Outgoing calls should report every call range within a caller.

```ds
function left(): void {}
function right(): void {}

function root(): void {
//       ^^^^ def:root
    left();
    right();
}
```

```query call_hierarchy def:root outgoing
main.ds:1:1-1:25 name=left kind=function selection=main.ds:1:10-1:14 calls=main.ds:5:5-5:11
main.ds:2:1-2:26 name=right kind=function selection=main.ds:2:10-2:15 calls=main.ds:6:5-6:12
```

## Cross Module

### Incoming calls across modules

Incoming calls should include callers from other modules.

```ds:lib.ds
export function sink(): void {
//              ^^^^ def:sink
}
```

```ds:impl.ds
import { sink } from "./lib.ds";

export function caller(): void {
    sink();
}
```

```query call_hierarchy def:sink incoming
caller
```

### Outgoing calls across modules

Outgoing calls should include callees from other modules.

```ds:lib.ds
export function helper(): void {}
```

```ds:main.ds
import { helper } from "./lib.ds";

function root(): void {
//       ^^^^ def:root
    helper();
}
```

```query call_hierarchy def:root outgoing
helper
```

## Recursion

### Recursive functions include self outgoing calls

Outgoing call hierarchy should include recursive self-calls.

```ds
function recur(): void {
//       ^^^^^ def:recur
    recur();
}
```

```query call_hierarchy def:recur outgoing
recur
```

### Recursive functions include self incoming calls

Incoming call hierarchy should include recursive self-calls.

```ds
function recur(): void {
//       ^^^^^ def:recur
    recur();
}
```

```query call_hierarchy def:recur incoming
recur
```

## Import Shapes

### Outgoing calls through namespace imports

Outgoing calls should resolve callee symbols through namespace imports.

```ds:lib.ds
export function helper(): void {}
```

```ds:main.ds
import * as tools from "./lib.ds";

function root(): void {
//       ^^^^ def:root
    tools.helper();
}
```

```query call_hierarchy def:root outgoing
helper
```

### Incoming calls through re-export chains

Incoming calls should resolve callers through re-exported import chains.

```ds:lib.ds
export function helper(): void {}
//              ^^^^^^ def:helper
```

```ds:barrel.ds
export { helper } from "./lib.ds";
```

```ds:main.ds
import { helper } from "./barrel.ds";

function root(): void {
    helper();
}
```

```query call_hierarchy def:helper incoming
root
```

## Empty Results

### Non-callable targets have no hierarchy

Non-callable symbols should not produce call hierarchy items.

```ds
struct $0Point {
    x: int32
}
```

```query call_hierarchy $0 incoming
<none>
```

```query call_hierarchy $0 outgoing
<none>
```

## Damaged Syntax

### Keep incoming calls after malformed call statements

Incoming call hierarchy should still resolve later valid callers after one malformed call statement.

```ds
broken(,

function target(): void {
//       ^^^^^^ def:target
}

function caller(): void {
    target();
}
```

```query call_hierarchy def:target incoming
caller
```

### Keep outgoing calls after bare new recovery statements

Outgoing call hierarchy should still resolve later valid callees after one bare `new` recovery statement.

```ds
new

function helper(): void {}

function root(): void {
//       ^^^^ def:root
    helper();
}
```

```query call_hierarchy def:root outgoing
helper
```
