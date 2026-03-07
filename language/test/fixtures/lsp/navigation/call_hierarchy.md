# Call Hierarchy

## Incoming Calls

### Calls from another module

Incoming call hierarchy should include calls from other files in the workspace.

```ds:lib.ds
export function /*call_hierarchy*/sink(): void {
}
```

```ds:impl.ds
import { sink } from "./lib.ds";

export function caller(): void {
    sink();
}
```

```lsp call_hierarchy_incoming
item=impl.ds:3:1-5:2
name=caller
kind=function
selection=impl.ds:3:17-3:23
call=impl.ds:4:5-4:11
```

### Grow and prune incoming callers across four states
Incoming call hierarchy should track each caller as files join and leave the call set.

```ds:lib.ds
export function /*call_hierarchy*/sink(): void {}
```

```ds:a.ds
import { sink } from "./lib.ds";

export function callerA(): void { sink(); }
```

```ds:b.ds
import { sink } from "./lib.ds";
```

```ds:c.ds
import { sink } from "./lib.ds";
```

```ds:b.ds[1]
import { sink } from "./lib.ds";

export function callerB(): void { sink(); }
```

```ds:a.ds[2]
export const idle = 1;
```

```ds:b.ds[2]
import { sink } from "./lib.ds";

export function callerB(): void { sink(); }
```

```ds:c.ds[3]
import { sink } from "./lib.ds";

export function callerC(): void { sink(); }
```

```lsp call_hierarchy_incoming call_hierarchy [0]
item=a.ds:3:1-3:44
name=callerA
kind=function
selection=a.ds:3:17-3:24
call=a.ds:3:35-3:41
```

```lsp call_hierarchy_incoming call_hierarchy [1]
item=a.ds:3:1-3:44
name=callerA
kind=function
selection=a.ds:3:17-3:24
call=a.ds:3:35-3:41

item=b.ds:3:1-3:44
name=callerB
kind=function
selection=b.ds:3:17-3:24
call=b.ds:3:35-3:41
```

```lsp call_hierarchy_incoming call_hierarchy [2]
item=b.ds:3:1-3:44
name=callerB
kind=function
selection=b.ds:3:17-3:24
call=b.ds:3:35-3:41
```

```lsp call_hierarchy_incoming call_hierarchy [3]
item=b.ds:3:1-3:44
name=callerB
kind=function
selection=b.ds:3:17-3:24
call=b.ds:3:35-3:41

item=c.ds:3:1-3:44
name=callerC
kind=function
selection=c.ds:3:17-3:24
call=c.ds:3:35-3:41
```

## Outgoing Calls

### Calls from one local function

Outgoing call hierarchy should include the exact local calls made by the selected function.

```ds:main.ds
function leaf(): void {
}

function /*call_hierarchy*/root(): void {
    leaf();
}
```

```lsp call_hierarchy_outgoing
item=main.ds:1:1-2:2
name=leaf
kind=function
selection=main.ds:1:10-1:14
call=main.ds:5:5-5:11
```

### Grow and prune outgoing calls across three states
Outgoing call hierarchy should reflect the exact callees after each overlay rewrite.

```ds:main.ds
function leafA(): void {}

function /*call_hierarchy*/root(): void { leafA(); }
```

```ds:main.ds[1]
function leafA(): void {}
function leafB(): void {}

function /*call_hierarchy*/root(): void { leafA(); leafB(); }
```

```ds:main.ds[2]
function leafA(): void {}
function leafB(): void {}

function /*call_hierarchy*/root(): void { leafB(); }
```

```lsp call_hierarchy_outgoing call_hierarchy [0]
item=main.ds:1:1-1:26
name=leafA
kind=function
selection=main.ds:1:10-1:15
call=main.ds:3:25-3:32
```

```lsp call_hierarchy_outgoing call_hierarchy [1]
item=main.ds:1:1-1:26
name=leafA
kind=function
selection=main.ds:1:10-1:15
call=main.ds:4:25-4:32

item=main.ds:2:1-2:26
name=leafB
kind=function
selection=main.ds:2:10-2:15
call=main.ds:4:34-4:41
```

```lsp call_hierarchy_outgoing call_hierarchy [2]
item=main.ds:2:1-2:26
name=leafB
kind=function
selection=main.ds:2:10-2:15
call=main.ds:4:25-4:32
```
