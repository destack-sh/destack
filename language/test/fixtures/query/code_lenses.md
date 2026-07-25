# Code Lenses

## References

### Offer function references

Functions receive a reference lens with the exact declaration and current reference count.

```ds main.ds
function ping(): void {}
         ^^^^ declaration

ping();
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=references count=1 symbol=main.ds#ping@1
```

## Implementations

### Offer interface implementations and class subclasses

Interfaces and classes receive implementation lenses with their direct implementation counts.

```ds main.ds
interface Animal {
          ^^^^^^ animal
    speak(): void;
}

class Dog implements Animal {
      ^^^ dog
    speak(): void {}
}

class Puppy extends Dog {}
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#animal action=implementations count=1 symbol=main.ds#Animal@1
@code_lenses.lens range=main.ds#dog action=implementations count=1 symbol=main.ds#Dog@4
```

### Offer nominal interface implementations

A nominal interface receives the same implementation lens as a structural interface.

```ds main.ds
newtype interface Encode {
                  ^^^^^^ interface
    encode(): string;
}

struct Packet implements Encode {
    encode(): string {
        return "";
    }
}
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#interface action=implementations count=1 symbol=main.ds#Encode@1
```

## Tests

### [ignored] Offer test actions for a decorated function

A test decorator produces run and debug lenses for its function.

```ds main.ds
@test
function verify(): void {}
         ^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=run_test test=verify symbol=main.ds#verify@1
@code_lenses.lens range=main.ds#declaration action=debug_test test=verify symbol=main.ds#verify@1
```

## Modules

### Retain cross-module symbol identity

Lens discovery retains the declaration identity needed to resolve references across modules.

```ds library.ds
export function ping(): void {}
                ^^^^ declaration
```

```ds main.ds
import { ping } from "./library.ds";

ping();
```

```query code_lenses library.ds
@code_lenses.lens range=library.ds#declaration action=references count=2 symbol=library.ds#ping@1
```

## Zero Counts

### Offer unused function references

An unused function still receives a lens reporting zero references.

```ds main.ds
function unused(): void {}
         ^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=references count=0 symbol=main.ds#unused@1
```

### Offer unimplemented interfaces

An interface still receives a lens reporting zero implementations.

```ds main.ds
interface Unimplemented {}
          ^^^^^^^^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=implementations count=0 symbol=main.ds#Unimplemented@1
```

## Empty Results

### Omit declarations without a lens family

Variables do not receive code lenses.

```ds main.ds
const value = 1;
```

```query code_lenses main.ds
@code_lenses.none
```
