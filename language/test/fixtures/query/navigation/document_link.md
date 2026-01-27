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
main.ds:1:21-1:31 target=file:/test/query/import-links-import-paths-should-produce-links-0/foo.ds tooltip=Go to ./foo.ds
main.ds:2:21-2:31 target=file:/test/query/import-links-import-paths-should-produce-links-0/bar.ds tooltip=Go to ./bar.ds
```
