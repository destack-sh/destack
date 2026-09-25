
## References

### Offer function references

Functions receive a lens with their reference count.

```tspp main.tspp
function ping(): void {}
         ^^^^ declaration

ping();
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=references count=1
```

### Count current references

Reference lenses count calls after each edit.

```tspp main.tspp
function ping(): void {}
         ^^^^ declaration
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=references count=0
```

```tspp main.tspp change
function ping(): void {}
         ^^^^ declaration

ping();
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=references count=1
```

```diff main.tspp
@@ -4,1 +4,2 @@
 ping();
+ping();
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=references count=2
```

## Implementations

### Offer interface implementations and class subclasses

Interfaces and classes receive implementation lenses with their direct implementation counts.

```tspp main.tspp
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

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#animal action=implementations count=1
@code_lenses.lens range=main.tspp#dog action=implementations count=1
@code_lenses.lens range=main.tspp#puppy action=implementations count=0
```

### Offer nominal interface implementations

A nominal interface receives the same implementation lens as a structural interface.

```tspp main.tspp
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

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#interface action=implementations count=1
```

## Modules

### Count references across modules

The defining module lens includes references from importing modules.

```tspp library.tspp
export function ping(): void {}
                ^^^^ declaration
```

```tspp main.tspp
import { ping } from "./library.tspp";

ping();
```

```query code_lenses library.tspp
@code_lenses.lens range=library.tspp#declaration action=references count=2
```

## Zero Counts

### Offer unused function references

An unused function still receives a lens reporting zero references.

```tspp main.tspp
function unused(): void {}
         ^^^^^^ declaration
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=references count=0
```

### Offer unimplemented interfaces

An interface still receives a lens reporting zero implementations.

```tspp main.tspp
interface Unimplemented {}
          ^^^^^^^^^^^^^ declaration
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=implementations count=0
```

### Offer classes without subclasses

A class still receives a lens reporting zero implementations.

```tspp main.tspp
class Standalone {}
      ^^^^^^^^^^ declaration
```

```query code_lenses main.tspp
@code_lenses.lens range=main.tspp#declaration action=implementations count=0
```

## Empty Results

### Omit declarations without a lens family

Variable bindings and their lambda initializers do not receive code lenses.

```tspp main.tspp
const value = (): int32 => 1;
```

```query code_lenses main.tspp
@code_lenses.none
```
