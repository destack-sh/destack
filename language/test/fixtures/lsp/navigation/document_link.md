# Document Link

## Import Links

### Link import specifiers

Document links should expose import specifiers as exact file targets.

```ds:main.ds
import { foo } from "./foo.ds";
import { bar } from "./bar.ds";
```

```ds:foo.ds
export const foo = 1;
```

```ds:bar.ds
export const bar = 2;
```

```lsp document_link
range=0:20-0:30
target=foo.ds
tooltip=Go to ./foo.ds

range=1:20-1:30
target=bar.ds
tooltip=Go to ./bar.ds
```
