---
title: Nominal Interfaces
description: Nominal interfaces (traits).
---

# Nominal Interfaces

- nominal interfaces (traits): explicit conformance, like a Rust trait
- receiver qualifiers state access requirements; associated constants can select an implementation's required access
- borrow weakening permits calls to weaker receivers; it does not establish another trait conformance

- implementations always need exact receivers, so `clone(&immutable this)` is implemented as `clone(&immutable this)`
- generated methods copy Copy union values before payload calls and require non-aliasing "protection" for non-Copy inline payload borrows

```ds
newtype interface Add<T> {
    add(a: T, b: T): T;
}
```
<!--
## Placement

- see more in .. ??-space?
- `local` / `shared` constrain the whole implementing type; bases must agree
- fields / parameters are relative to their base
- individual methods can separately constrain parameters
- `SharedSafe` _permits_ shared use; it does not require shared placement

```ds
local newtype interface LocalService {}
class LocalServiceImpl implements LocalService {}

shared newtype interface SharedService {}
class SharedServiceImpl implements SharedService {}
```-->
