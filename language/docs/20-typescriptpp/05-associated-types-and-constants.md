---
title: Associated Types and Constants
description: Associated Types and Constants
---

# Associated Types and Constants

Associated types and constants contribute static members to a type that can be reused within the type and its implementors but do not need to be exposed to every single caller.
Both associated types and constants also work in abstract types, and as they are associated with the type directly, they do not occupy any instance space on the type.
All statically known types and constants share the same static evaluation logic, and thus associated types and constants also mix with generic parameters, conditional types, decorators, and so on.

## Types

```ds
import { IteratorResult } from "destack:iter";

newtype interface Collection {
    type Item;
    type Return = void;

    next(): IteratorResult<this.Item, this.Return>;
}

declare function collect<I: Collection>(iter: I): I.Item[];
```

Associated types are type aliases scoped to some struct, class, or interface and can also reference the owner's generic parameters.

```ds
interface BufferPool {
    type Buffer<T>;
    type Error;

    acquire<T>(count: usize): Result<this.Buffer<T>, this.Error>;
    release<T>(buffer: this.Buffer<T>): void;
}
```

Associated types can have their _own_ generic parameters with the same generic parameter forms as ordinary declarations, including type parameters and `const` parameters (these are generic associated types, often called GATs).

```ds
interface Storage {
    type Handle<T>;
    type Page<const Size: uint>;
}
```

## Constants

In addition to associated types, nominal type declarations also support associated constant members as static compile-time values.
Like `static` members, const members require no instance storage, but unlike `static` members, they are evaluated during compilation.

```ds
interface RegisterBlock {
    const Width: uint;

    read(): [uint8; this.Width];
    write(bytes: &[uint8; this.Width]): void;
}
```

Associated members participate in the same static evaluation / inference world, and so associated members can express dependent types and values that are dependent on others (including inferred!).

```ds
interface Matrix<Row> {
    const Width: uint = Row extends string ? 8 : 4;
    type Bytes = [uint8; this.Width];
}
```

## Refinements

Associated members (both types and constants) can be refined explicitly at application sites whenever an erased or constrained value needs a concrete associated type with `type Name = T` for types and `const Name = value` for constants.

```ds
declare function readBlock<T: RegisterBlock<const Width = 16>>(block: T): [uint8; 16];
```
