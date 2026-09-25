# Import Statement Sorting

Import sorting fixtures cover import groups, side effects, scoped packages, and relative paths.

## Group Ordering

### builtin imports first

Builtin imports (using protocol prefix like `node:`, `bun:`, `deno:`) come before external packages.

```tspp
{
    import lodash from "lodash"
    import fs from "node:fs"
    import path from "node:path"
}
```

```tspp expected
{
    import fs from "node:fs";
    import path from "node:path";

    import lodash from "lodash";
}
```

### multiple runtime protocols

Any protocol prefix (not just `node:`) is recognized as a builtin.

```tspp
{
    import lodash from "lodash"
    import test from "bun:test"
    import fs from "node:fs"
}
```

```tspp expected
{
    import test from "bun:test";
    import fs from "node:fs";

    import lodash from "lodash";
}
```

### packages before relative

External packages come before relative imports.

```tspp
{
    import local from "./local"
    import lodash from "lodash"
}
```

```tspp expected
{
    import lodash from "lodash";

    import local from "./local";
}
```

### aliases before relative

Path aliases come before relative imports.

```tspp
{
    import local from "./local"
    import util from "@/utils"
}
```

```tspp expected
{
    import util from "@/utils";

    import local from "./local";
}
```

## alphabetical within groups

### packages sorted

Packages are sorted alphabetically within their group.

```tspp
{
    import zod from "zod"
    import axios from "axios"
    import lodash from "lodash"
}
```

```tspp expected
{
    import axios from "axios";
    import lodash from "lodash";
    import zod from "zod";
}
```

### relative imports sorted

Relative imports are sorted alphabetically.

```tspp
{
    import z from "./z"
    import a from "./a"
    import m from "../m"
}
```

```tspp expected
{
    import m from "../m";
    import a from "./a";
    import z from "./z";
}
```

## side-effect imports

### side-effects stay at top

Side-effect imports are not reordered and stay at the top.

```tspp
{
    import "./setup"
    import lodash from "lodash"
    import "./polyfill"
}
```

```tspp expected
{
    import "./setup";
    import "./polyfill";

    import lodash from "lodash";
}
```

## scoped packages

### scoped packages as packages

Scoped packages like `@org/pkg` are treated as regular packages.

```tspp
{
    import local from "./local"
    import scoped from "@org/package"
    import lodash from "lodash"
}
```

```tspp expected
{
    import scoped from "@org/package";
    import lodash from "lodash";

    import local from "./local";
}
```

## full example

### comprehensive import sorting

All groups in their correct order.

```tspp
{
    import "./polyfill"
    import local from "./components/Button"
    import React from "react"
    import path from "node:path"
    import util from "@/utils"
    import fs from "node:fs"
    import lodash from "lodash"
    import parent from "../parent"
}
```

```tspp expected
{
    import "./polyfill";

    import fs from "node:fs";
    import path from "node:path";

    import lodash from "lodash";
    import React from "react";

    import util from "@/utils";

    import parent from "../parent";
    import local from "./components/Button";
}
```
