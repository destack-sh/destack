# Code Lenses

## References

### Offer function references

Functions receive a lens with their reference count.

```ds main.ds
function ping(): void {}
         ^^^^ declaration

ping();
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=references count=1
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
      ^^^^^ puppy
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#animal action=implementations count=1
@code_lenses.lens range=main.ds#dog action=implementations count=1
@code_lenses.lens range=main.ds#puppy action=implementations count=0
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
@code_lenses.lens range=main.ds#interface action=implementations count=1
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
@code_lenses.lens range=main.ds#declaration action=run_test symbol=main.ds#verify@1
@code_lenses.lens range=main.ds#declaration action=debug_test symbol=main.ds#verify@1
```

## Modules

### Count references across modules

The defining module lens includes references from importing modules.

```ds library.ds
export function ping(): void {}
                ^^^^ declaration
```

```ds main.ds
import { ping } from "./library.ds";

ping();
```

```query code_lenses library.ds
@code_lenses.lens range=library.ds#declaration action=references count=2
```

## Zero Counts

### Offer unused function references

An unused function still receives a lens reporting zero references.

```ds main.ds
function unused(): void {}
         ^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=references count=0
```

### Offer unimplemented interfaces

An interface still receives a lens reporting zero implementations.

```ds main.ds
interface Unimplemented {}
          ^^^^^^^^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=implementations count=0
```

### Offer classes without subclasses

A class still receives a lens reporting zero implementations.

```ds main.ds
class Standalone {}
      ^^^^^^^^^^ declaration
```

```query code_lenses main.ds
@code_lenses.lens range=main.ds#declaration action=implementations count=0
```

## Empty Results

### Omit declarations without a lens family

Variable bindings and their lambda initializers do not receive code lenses.

```ds main.ds
const value = (): int32 => 1;
```

```query code_lenses main.ds
@code_lenses.none
```
