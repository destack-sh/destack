# Goto Definition

## Imported Values

### Jump from an imported use site

Goto definition should jump from an imported use site to the exported declaration.

```ds:main.ds
import { ping } from "./lib.ds";
const result = /*use*/ping();
```

```ds:lib.ds
export function /*def*/ping(): void {}
```

## Repeated Lookups

### Each marker across modules

Goto definition should resolve each marked use site to the shared declaration across modules.

```ds:lib.ds
export const /*def*/value = 1;
```

```ds:main.ds
import { value } from "./lib.ds";
const first = /*use_first*/value;
const second = /*use_second*/value;
```

```lsp scenario definition-each-marker-cross-module
```

### Each marker in one file

Goto definition should resolve each marked use site to the local declaration in the same file.

```ds:main.ds
const [|value|] = 1;
const first = /*use_first*/value;
const second = /*use_second*/value;
```

```lsp scenario definition-each-marker-same-file
```

