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
