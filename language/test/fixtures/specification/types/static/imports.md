# Imports

Imports can use exported static results.
They cannot refine an exported declaration from the importing module.

## exports

### imports cannot refine exports

An importing module cannot turn `uint` into the literal `3`.

```ds:a.ds
export declare const N: uint;
```

```ds:b.ds
import { N } from "./a.ds";

N satisfies 3;
```

- contains: not assignable
