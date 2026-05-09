# Document Links

## Import Links

### Import paths should produce links

Document links should include import specifiers.

```ds:main.ds
$0import { foo } from "./foo.ds";
import { bar } from "./bar.ds";
```

```ds:foo.ds
export const foo = 1;
```

```ds:bar.ds
export const bar = 2;
```

```query document_link $0
main.ds:1:21-1:31 target=file:foo.ds tooltip=Go to ./foo.ds
main.ds:2:21-2:31 target=file:bar.ds tooltip=Go to ./bar.ds
```

## Resolve Links

### Resolving links keeps the same target

Resolved document links should keep their target when already resolved.

```ds:main.ds
import { foo } from "./foo.ds";
import { bar } from "./b$0ar.ds";
```

```ds:foo.ds
export const foo = 1;
```

```ds:bar.ds
export const bar = 2;
```

```query resolve_document_link $0
<same>
```

## Re-Exports

### Re-export specifiers should produce links

Document links should include re-export specifiers.

```ds:main.ds
$0export { foo } from "./foo.ds";
```

```ds:foo.ds
export const foo = 1;
```

```query document_link $0
main.ds:1:21-1:31 target=file:foo.ds tooltip=Go to ./foo.ds
```

## Export Stars

### Export star specifiers should produce links

Document links should include export star specifiers.

```ds:main.ds
$0export * from "./foo.ds";
```

```ds:foo.ds
export const foo = 1;
```

```query document_link $0
main.ds:1:15-1:25 target=file:foo.ds tooltip=Go to ./foo.ds
```

## Mixed Links

### Imports and re-exports should both link

Document links should include both import and re-export specifiers in the same file.

```ds:main.ds
$0import { foo } from "./foo.ds";
export { bar } from "./bar.ds";
```

```ds:foo.ds
export const foo = 1;
```

```ds:bar.ds
export const bar = 2;
```

```query document_link $0
main.ds:1:21-1:31 target=file:foo.ds tooltip=Go to ./foo.ds
main.ds:2:21-2:31 target=file:bar.ds tooltip=Go to ./bar.ds
```

### Ignore non import string literals

Document links should ignore regular string literals that are not module specifiers.

```ds
const path = "./foo.ds";
```

```query document_link $0
<none>
```

## Side-Effect Imports

### Side-effect imports should produce links

Document links should include side-effect import specifiers.

```ds:main.ds
$0import "./setup.ds";
```

```ds:setup.ds
export const ready = true;
```

```query document_link $0
main.ds:1:8-1:20 target=file:setup.ds tooltip=Go to ./setup.ds
```

## Unresolved Specifiers

### Surface unresolved module specifiers

Document links should still surface unresolved module specifiers as file targets.

```ds:main.ds
$0import { missing } from "./missing.ds";
```

```query document_link $0
main.ds:1:25-1:39 target=file:missing.ds tooltip=Go to ./missing.ds
```

## Damaged Syntax

### Keep links after malformed import clauses

Document links should still resolve later valid specifiers after one malformed import clause.

```ds:main.ds
import { from "./broken.ds";
$0import { foo } from "./foo.ds";
```

```ds:foo.ds
export const foo = 1;
```

```query document_link $0
main.ds:2:21-2:31 target=file:foo.ds tooltip=Go to ./foo.ds
```
