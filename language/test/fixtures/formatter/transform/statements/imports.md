# Import Statements

Tests for import statement formatting.

## Named Imports

### import braces have internal spacing

Import braces get internal spacing, like object literals.

```ds
import {foo,bar,baz} from "module"
```

Spaces are added after `{` and before `}`.

```ds expected
import { foo, bar, baz } from "module";
```

### short imports stay on one line

Short import lists remain on a single line.

```ds
import { a, b, c } from "module"
```

```ds expected
import { a, b, c } from "module";
```
