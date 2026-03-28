# Rename

## Preparation

### Prepare rename at a local value

Prepare rename should return the exact editable span for a local binding.

```ds:main.ds
const [|/*prepare_rename*/value|] = 1;
const next = value + 1;
```

### Prepare rename at an imported alias

Prepare rename should return the exact local alias span for aliased imports.

```ds:lib.ds
export function greet(name: string): string {
    return name;
}
```

```ds:main.ds
import { greet as [|/*prepare_rename*/localGreet|] } from "./lib.ds";

const output = localGreet("Ada");
```

## Cross-Module Updates

### Rename an exported symbol across modules

Rename should update the exported symbol, its imports, and its uses across files.

```ds:main.ds
import { [|greet|] } from "./lib.ds";
const output = [|greet|]("Ada");
```

```ds:lib.ds
export function [|greet|](name: string): string {
    return name;
}
```

### Rename an exported symbol through direct, namespace, and barrel consumers

Rename should update every rewritten token across direct imports, namespace member uses, and barrel re-exports.

```ds:lib.ds
export function [|/*rename*/ping|](): void {}
```

```ds:barrel.ds
export { [|ping|] } from "./lib.ds";
```

```ds:main_named.ds
import { [|ping|] } from "./lib.ds";

[|ping|]();
```

```ds:main_namespace.ds
import * as api from "./lib.ds";

api.[|ping|]();
```

```ds:main_barrel.ds
import { [|ping|] } from "./barrel.ds";

[|ping|]();
```
