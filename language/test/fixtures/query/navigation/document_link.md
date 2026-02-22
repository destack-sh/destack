# Document Links

## Import Links

### Import paths should produce links

Document links should include import and re-export specifiers.

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

## Type Import Expressions

### Type import expressions should produce links

Document links should include `import()` type expressions.

```ds:main.ds
$0type Foo = import("./types/foo").Foo;
```

```ds:types/foo.ds
export type Foo = {
    value: int32,
};
```

```query document_link $0
main.ds:1:19-1:32 target=file:types/foo.ds tooltip=Go to ./types/foo
```

### Ignore non import string literals

Document links should ignore regular string literals that are not module specifiers.

```ds
const path = "./foo.ds";
```

```query document_link $0
<none>
```
