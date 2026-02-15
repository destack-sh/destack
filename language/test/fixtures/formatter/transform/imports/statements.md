# Import Statement Sorting

Tests for sorting import statements by group and alphabetically.

## group ordering

### builtin imports first

Builtin imports (using protocol prefix like `node:`, `bun:`, `deno:`) come before external packages.

```ds organize-imports=on
{
    import lodash from "lodash"
    import fs from "node:fs"
    import path from "node:path"
}
```

```ds expected
{
    import lodash from "lodash";
    import fs from "node:fs";
    import path from "node:path";
}
```

### multiple runtime protocols

Any protocol prefix (not just `node:`) is recognized as a builtin.

```ds organize-imports=on
{
    import lodash from "lodash"
    import test from "bun:test"
    import fs from "node:fs"
}
```

```ds expected
{
    import lodash from "lodash";
    import test from "bun:test";
    import fs from "node:fs";
}
```

### packages before relative

External packages come before relative imports.

```ds organize-imports=on
{
    import local from "./local"
    import lodash from "lodash"
}
```

```ds expected
{
    import local from "./local";
    import lodash from "lodash";
}
```

### aliases before relative

Path aliases come before relative imports.

```ds organize-imports=on
{
    import local from "./local"
    import util from "@/utils"
}
```

```ds expected
{
    import local from "./local";
    import util from "@/utils";
}
```

## alphabetical within groups

### packages sorted

Packages are sorted alphabetically within their group.

```ds organize-imports=on
{
    import zod from "zod"
    import axios from "axios"
    import lodash from "lodash"
}
```

```ds expected
{
    import zod from "zod";
    import axios from "axios";
    import lodash from "lodash";
}
```

### relative imports sorted

Relative imports are sorted alphabetically.

```ds organize-imports=on
{
    import z from "./z"
    import a from "./a"
    import m from "../m"
}
```

```ds expected
{
    import z from "./z";
    import a from "./a";
    import m from "../m";
}
```

## side-effect imports

### side-effects stay at top

Side-effect imports are not reordered and stay at the top.

```ds organize-imports=on
{
    import "./setup"
    import lodash from "lodash"
    import "./polyfill"
}
```

```ds expected
{
    import "./setup";
    import lodash from "lodash";
    import "./polyfill";
}
```

## scoped packages

### scoped packages as packages

Scoped packages like `@org/pkg` are treated as regular packages.

```ds organize-imports=on
{
    import local from "./local"
    import scoped from "@org/package"
    import lodash from "lodash"
}
```

```ds expected
{
    import local from "./local";
    import scoped from "@org/package";
    import lodash from "lodash";
}
```

## full example

### comprehensive import sorting

All groups in their correct order.

```ds organize-imports=on
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

```ds expected
{
    import "./polyfill";
    import local from "./components/Button";
    import React from "react";
    import path from "node:path";
    import util from "@/utils";
    import fs from "node:fs";
    import lodash from "lodash";
    import parent from "../parent";
}
```
