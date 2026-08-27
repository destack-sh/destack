
## Empty Results

### Return no decorators from an undecorated module

An undecorated module has no decorator results.

```ds main.ds
export const value = 1;
```

```query decorators scope=main.ds
@decorators.none
```

## Module

### Return an application and its owner

Module scope returns the language item and decorated declaration.

```ds main.ds
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verify(): void {}
         ^^^^^^ target
```

```query decorators scope=main.ds
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.ds#decorator node=main.ds#decorator@4
@decorators.owner index=0 location=main.ds#target node=main.ds#declaration@8
```

```query decorators scope=main.ds name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.ds#decorator node=main.ds#decorator@4
@decorators.owner index=0 location=main.ds#target node=main.ds#declaration@8
```

### Return a user-defined annotation

A user-defined annotation returns its declaration symbol.

```ds main.ds
newtype tracked = ();

@tracked
 ^^^^^^^ decorator
class Service {}
      ^^^^^^^ target
```

```query decorators scope=main.ds name=tracked
@decorators.application index=0 name=tracked role=symbol symbol=main.ds#tracked@1 location=main.ds#decorator node=main.ds#decorator@4
@decorators.owner index=0 location=main.ds#target node=main.ds#declaration@5
```

### Preserve application source order

Unfiltered results follow source order on a shared owner.

```ds main.ds
newtype tracked = ();

@tracked
 ^^^^^^^ first
@deprecated("use ServiceV2")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ second
class Service {}
      ^^^^^^^ target
```

```query decorators scope=main.ds
@decorators.application index=0 name=tracked role=symbol symbol=main.ds#tracked@1 location=main.ds#first node=main.ds#decorator@4
@decorators.owner index=0 location=main.ds#target node=main.ds#declaration@10
@decorators.application index=1 name=deprecated role=language_item language_item=deprecated location=main.ds#second node=main.ds#decorator@9
@decorators.owner index=1 location=main.ds#target node=main.ds#declaration@10
```

### Return current decorator applications

Decorator lookup includes applications added in later revisions.

```ds main.ds
function verify(): void {}
```

```query decorators scope=main.ds
@decorators.none
```

```ds main.ds change
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verify(): void {}
         ^^^^^^ target
```

```query decorators scope=main.ds
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=main.ds#decorator node=main.ds#decorator@4
@decorators.owner index=0 location=main.ds#target node=main.ds#declaration@8
```

## Program

### Return decorators across modules

Program scope returns matching applications in stable module order.

```ds alpha.ds
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verifyAlpha(): void {}
         ^^^^^^^^^^^ target
```

```ds beta.ds
@deprecated("use verifyNew")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ decorator
function verifyBeta(): void {}
         ^^^^^^^^^^ target
```

```query decorators scope=program name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=alpha.ds#decorator node=alpha.ds#decorator@4
@decorators.owner index=0 location=alpha.ds#target node=alpha.ds#declaration@8
@decorators.application index=1 name=deprecated role=language_item language_item=deprecated location=beta.ds#decorator node=beta.ds#decorator@4
@decorators.owner index=1 location=beta.ds#target node=beta.ds#declaration@8
```

```query decorators scope=beta.ds name=deprecated
@decorators.application index=0 name=deprecated role=language_item language_item=deprecated location=beta.ds#decorator node=beta.ds#decorator@4
@decorators.owner index=0 location=beta.ds#target node=beta.ds#declaration@8
```

## Name Filter

### Return no decorators for an unmatched name

The name filter excludes every nonmatching application.

```ds main.ds
@deprecated("use verifyNew")
function verify(): void {}
```

```query decorators scope=main.ds name=missing
@decorators.none
```
