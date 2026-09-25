
## Empty Results

### Return no decorators from an undecorated module

An undecorated module has no decorator results.

```tspp main.tspp
export const value = 1;
```

```query decorators scope=main.tspp
@decorators.none
```

## Module

### Return an application and its owner

Module scope returns the language item and decorated declaration.

```tspp main.tspp
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verify(): void {}
         ^^^^^^ target
```

```query decorators scope=main.tspp
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.tspp#decorator node=main.tspp#decorator@4
@decorators.owner index=0 location=main.tspp#target node=main.tspp#declaration@8
```

```query decorators scope=main.tspp name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.tspp#decorator node=main.tspp#decorator@4
@decorators.owner index=0 location=main.tspp#target node=main.tspp#declaration@8
```

### Return a user-defined annotation

A user-defined annotation returns its declaration symbol.

```tspp main.tspp
newtype tracked = ();

@tracked
 ^^^^^^^ decorator
class Service {}
      ^^^^^^^ target
```

```query decorators scope=main.tspp name=tracked
@decorators.application index=0 name=tracked role=symbol symbol=main.tspp#tracked@1 location=main.tspp#decorator node=main.tspp#decorator@4
@decorators.owner index=0 location=main.tspp#target node=main.tspp#declaration@5
```

### Preserve application source order

Unfiltered results follow source order on a shared owner.

```tspp main.tspp
newtype tracked = ();

@tracked
 ^^^^^^^ first
@deprecated("use ServiceV2")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ second
class Service {}
      ^^^^^^^ target
```

```query decorators scope=main.tspp
@decorators.application index=0 name=tracked role=symbol symbol=main.tspp#tracked@1 location=main.tspp#first node=main.tspp#decorator@4
@decorators.owner index=0 location=main.tspp#target node=main.tspp#declaration@10
@decorators.application index=1 name=deprecated role=language_item language_item=deprecated location=main.tspp#second node=main.tspp#decorator@9
@decorators.owner index=1 location=main.tspp#target node=main.tspp#declaration@10
```

### Return current decorator applications

Decorator lookup includes applications added by later edits.

```tspp main.tspp
function verify(): void {}
```

```query decorators scope=main.tspp
@decorators.none
```

```tspp main.tspp change
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verify(): void {}
         ^^^^^^ target
```

```query decorators scope=main.tspp
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.tspp#decorator node=main.tspp#decorator@4
@decorators.owner index=0 location=main.tspp#target node=main.tspp#declaration@8
```

## Program

### Return decorators across modules

Program scope returns matching applications in stable module order.

```tspp alpha.tspp
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verifyAlpha(): void {}
         ^^^^^^^^^^^ target
```

```tspp beta.tspp
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verifyBeta(): void {}
         ^^^^^^^^^^ target
```

```query decorators scope=program name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=alpha.tspp#decorator node=alpha.tspp#decorator@4
@decorators.owner index=0 location=alpha.tspp#target node=alpha.tspp#declaration@8
@decorators.application index=1 name=deprecated role=language_item language_item=deprecated location=beta.tspp#decorator node=beta.tspp#decorator@4
@decorators.owner index=1 location=beta.tspp#target node=beta.tspp#declaration@8
```

```query decorators scope=beta.tspp name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=beta.tspp#decorator node=beta.tspp#decorator@4
@decorators.owner index=0 location=beta.tspp#target node=beta.tspp#declaration@8
```

## Name Filter

### Return no decorators for an unmatched name

The name filter excludes every nonmatching application.

```tspp main.tspp
@deprecated("use verifyNew")
function verify(): void {}
```

```query decorators scope=main.tspp name=missing
@decorators.none
```
