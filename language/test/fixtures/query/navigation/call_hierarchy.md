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
main.ds:4:1-6:2 name=caller kind=function selection=main.ds:4:10-4:16 calls=main.ds:5:5-5:11|main.ds:5:11-5:13
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
main.ds:4:1-7:2 name=caller kind=function selection=main.ds:4:10-4:16 calls=main.ds:5:5-5:11|main.ds:5:11-5:13|main.ds:6:5-6:11|main.ds:6:11-6:13
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
main.ds:1:1-2:2 name=leaf kind=function selection=main.ds:1:10-1:14 calls=main.ds:5:9-5:11
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
main.ds:1:1-1:25 name=left kind=function selection=main.ds:1:10-1:14 calls=main.ds:5:9-5:11
main.ds:2:1-2:26 name=right kind=function selection=main.ds:2:10-2:15 calls=main.ds:6:10-6:12
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
