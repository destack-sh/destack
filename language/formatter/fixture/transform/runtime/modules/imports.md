# Import Statements

Import statement fixtures cover import syntax forms, attributes, comments, and long module specifiers.

## Named Imports

### import braces have internal spacing

Import braces get internal spacing, like object literals.

```tspp
import {foo,bar,baz} from "module"
```

Spaces are added after `{` and before `}`.

```tspp expected
import { bar, baz, foo } from "module";
```

### short imports stay on one line

Short import lists remain on a single line.

```tspp
import { a, b, c } from "module"
```

```tspp expected
import { a, b, c } from "module";
```

### single named import

Single imports also get internal spacing.

```tspp
import { foo } from "module"
```

```tspp expected
import { foo } from "module";
```

### import with alias

Imports can rename values using `as`.

```tspp
import { foo as bar } from "module"
```

```tspp expected
import { foo as bar } from "module";
```

### multiple imports with aliases

Multiple imports can each have aliases.

```tspp
import { foo as f, bar as b, baz as z } from "module"
```

```tspp expected
import { bar as b, foo as f, baz as z } from "module";
```

## Default Imports

### default import

Default imports use the value directly without braces.

```tspp
import foo from "module"
```

```tspp expected
import foo from "module";
```

### default and named imports

Default and named imports can be combined.

```tspp
import foo, { bar, baz } from "module"
```

```tspp expected
import foo, { bar, baz } from "module";
```

### default with alias

Both default and named imports support aliases.

```tspp
import Default, { foo as f } from "module"
```

```tspp expected
import Default, { foo as f } from "module";
```

## Namespace Imports

### namespace import

Namespace imports collect all exports under a single identifier.

```tspp
import * as mod from "module"
```

```tspp expected
import * as mod from "module";
```

### namespace with extra spacing

Extra spacing is normalized to single spaces.

```tspp
import   *   as   mod   from   "module"
```

```tspp expected
import * as mod from "module";
```

## Side Effect Imports

### side effect import

Side effect imports execute a module without importing bindings.

```tspp
import "module"
```

```tspp expected
import "module";
```

### side effect with extra spacing

Extra spacing after `import` is removed.

```tspp
import   "module"
```

```tspp expected
import "module";
```

## Line Breaking

### long import breaks

When imports exceed the line width, they break to multiple lines.

```tspp line-width=40
import { veryLongName, anotherLongName, thirdLongName } from "module"
```

Each import goes on its own line with a trailing comma.

```tspp expected
import {
    anotherLongName,
    thirdLongName,
    veryLongName,
} from "module";
```

### many imports break

Many short imports also break when they exceed the line width.

```tspp line-width=50
import { a, b, c, d, e, f, g, h, i, j, k } from "module"
```

```tspp expected
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

```tspp
import { foo } from "@organization/very-long-package-name/deeply/nested/module"
```

```tspp expected
import { foo } from "@organization/very-long-package-name/deeply/nested/module";
```

## Re-exports

### re-export all

All exports from a module can be re-exported.

```tspp
export * from "module"
```

```tspp expected
export * from "module";
```

### re-export named

Specific exports can be selected for re-export.

```tspp
export { foo, bar } from "module"
```

```tspp expected
export { bar, foo } from "module";
```

### re-export with alias

Re-exports can be renamed using `as`.

```tspp
export { foo as f, bar as b } from "module"
```

```tspp expected
export { bar as b, foo as f } from "module";
```

### re-export as namespace

All exports can be bundled under a namespace.

```tspp
export * as ns from "module"
```

```tspp expected
export * as ns from "module";
```

## Import Attributes

### import with attributes

Import attributes provide metadata about the module.

```tspp
import data from "data.json" with { type: "json" }
```

```tspp expected
import data from "data.json" with { type: "json" };
```

### import with multiple attributes

Multiple attributes can be specified.

```tspp
import styles from "styles.css" with { type: "css", scope: "local" }
```

```tspp expected
import styles from "styles.css" with { type: "css", scope: "local" };
```

### re-export with attributes

Re-exports can include module attributes.

```tspp
export { foo } from "data.json" with { type: "json" }
```

```tspp expected
export { foo } from "data.json" with { type: "json" };
```
