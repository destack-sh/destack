# Import Statement Sorting

Tests for sorting import statements by group and alphabetically.

## group ordering

### node builtins first

Node.js builtins come before external packages.

```ds organize-imports=on
{
    import lodash from "lodash"
    import fs from "fs"
    import path from "path"
}
```

```ds expected
{
    import fs from "fs";
    import path from "path";

    import lodash from "lodash";
}
```

### node protocol

The `node:` protocol is recognized as a builtin.

```ds organize-imports=on
{
    import lodash from "lodash"
    import fs from "node:fs"
}
```

```ds expected
{
    import fs from "node:fs";

    import lodash from "lodash";
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
    import lodash from "lodash";

    import local from "./local";
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
    import util from "@/utils";

    import local from "./local";
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
    import axios from "axios";
    import lodash from "lodash";
    import zod from "zod";
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
    import m from "../m";
    import a from "./a";
    import z from "./z";
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
    import "./polyfill";

    import lodash from "lodash";
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
    import scoped from "@org/package";
    import lodash from "lodash";

    import local from "./local";
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
    import path from "path"
    import util from "@/utils"
    import fs from "node:fs"
    import lodash from "lodash"
    import parent from "../parent"
}
```

```ds expected
{
    import "./polyfill";

    import fs from "node:fs";
    import path from "path";

    import lodash from "lodash";
    import React from "react";

    import util from "@/utils";

    import parent from "../parent";
    import local from "./components/Button";
}
```
