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
