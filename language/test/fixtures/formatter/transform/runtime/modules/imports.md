# Import Statements

Import statement fixtures cover import syntax forms, attributes, comments, and long module specifiers.

## Named Imports

### import braces have internal spacing

Import braces get internal spacing, like object literals.

```ds
import {foo,bar,baz} from "module"
```

Spaces are added after `{` and before `}`.

```ds expected
import { bar, baz, foo } from "module";
```

### short imports stay on one line

Short import lists remain on a single line.

```ds
import { a, b, c } from "module"
```

```ds expected
import { a, b, c } from "module";
```

### single named import

Single imports also get internal spacing.

```ds
import { foo } from "module"
```

```ds expected
import { foo } from "module";
```

### import with alias

Imports can rename values using `as`.

```ds
import { foo as bar } from "module"
```

```ds expected
import { foo as bar } from "module";
```

### multiple imports with aliases

Multiple imports can each have aliases.

```ds
import { foo as f, bar as b, baz as z } from "module"
```

```ds expected
import { bar as b, foo as f, baz as z } from "module";
```

## Default Imports

### default import

Default imports use the value directly without braces.

```ds
import foo from "module"
```

```ds expected
import foo from "module";
```

### default and named imports

Default and named imports can be combined.

```ds
import foo, { bar, baz } from "module"
```

```ds expected
import foo, { bar, baz } from "module";
```

### default with alias

Both default and named imports support aliases.

```ds
import Default, { foo as f } from "module"
```

```ds expected
import Default, { foo as f } from "module";
```

## Namespace Imports

### namespace import

Namespace imports collect all exports under a single identifier.

```ds
import * as mod from "module"
```

```ds expected
import * as mod from "module";
```

### namespace with extra spacing

Extra spacing is normalized to single spaces.

```ds
import   *   as   mod   from   "module"
```

```ds expected
import * as mod from "module";
```

## Side Effect Imports

### side effect import

Side effect imports execute a module without importing bindings.

```ds
import "module"
```

```ds expected
import "module";
```

### side effect with extra spacing

Extra spacing after `import` is removed.

```ds
import   "module"
```

```ds expected
import "module";
```

## Type Imports

### type-only import

Type-only imports use `import type`.

```ds
import type { Foo } from "module"
```

```ds expected
import type { Foo } from "module";
```

### type-only namespace import

Namespace imports can also be type-only.

```ds
import type * as Types from "module"
```

```ds expected
import type * as Types from "module";
```

### mixed type and value imports

Type and value imports can be mixed using `type` modifier on individual items.

```ds
import { type Foo, bar } from "module"
```

```ds expected
import { type Foo, bar } from "module";
```

### multiple type imports

Multiple types can be imported alongside values.

```ds
import { type Foo, type Bar, baz } from "module"
```

```ds expected
import { type Bar, type Foo, baz } from "module";
```

## Line Breaking

### long import breaks

When imports exceed the line width, they break to multiple lines.

```ds line-width=40
import { veryLongName, anotherLongName, thirdLongName } from "module"
```

Each import goes on its own line with a trailing comma.

```ds expected
import {
    anotherLongName,
    thirdLongName,
    veryLongName,
} from "module";
```

### many imports break

Many short imports also break when they exceed the line width.

```ds line-width=50
import { a, b, c, d, e, f, g, h, i, j, k } from "module"
```

```ds expected
import {
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h,
    i,
    j,
    k,
} from "module";
```

### long module path

Long module paths are preserved as-is.

```ds
import { foo } from "@organization/very-long-package-name/deeply/nested/module"
```

```ds expected
import { foo } from "@organization/very-long-package-name/deeply/nested/module";
```

## Re-exports

### re-export all

All exports from a module can be re-exported.

```ds
export * from "module"
```

```ds expected
export * from "module";
```

### re-export named

Specific exports can be selected for re-export.

```ds
export { foo, bar } from "module"
```

```ds expected
export { bar, foo } from "module";
```

### re-export with alias

Re-exports can be renamed using `as`.

```ds
export { foo as f, bar as b } from "module"
```

```ds expected
export { bar as b, foo as f } from "module";
```

### re-export as namespace

All exports can be bundled under a namespace.

```ds
export * as ns from "module"
```

```ds expected
export * as ns from "module";
```

### type re-export

Type-only re-exports use `export type` (parser issue).

```ds
export type { Foo, Bar } from "module"
```

```ds expected
export type { Bar, Foo } from "module";
```

## Import Attributes

### import with attributes

Import attributes provide metadata about the module.

```ds
import data from "data.json" with { type: "json" }
```

```ds expected
import data from "data.json" with { type: "json" };
```

### import with multiple attributes

Multiple attributes can be specified.

```ds
import styles from "styles.css" with { type: "css", scope: "local" }
```

```ds expected
import styles from "styles.css" with { type: "css",scope: "local" };
```

### re-export with attributes

Re-exports can include module attributes.

```ds
export { foo } from "data.json" with { type: "json" }
```

```ds expected
export { foo } from "data.json" with { type: "json" };
```
