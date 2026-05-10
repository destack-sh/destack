# Import Specifier Sorting

Import specifier fixtures cover sorting inside import and export braces.

## Specifier Sorting

### alphabetical order

Import specifiers should be sorted alphabetically.

```ds organize-imports=on
import { zebra, apple, mango } from "fruits"
```

```ds expected
import { apple, mango, zebra } from "fruits";
```

### with default import

Default import stays first, remaining specifiers are sorted.

```ds organize-imports=on
import Default, { zebra, apple, mango } from "fruits"
```

```ds expected
import Default, { apple, mango, zebra } from "fruits";
```

### natural sort order

Numbers are sorted as integers, not lexicographically.

```ds organize-imports=on
import { item10, item2, item1 } from "items"
```

```ds expected
import { item1, item2, item10 } from "items";
```

## type imports

### type before value

Type imports come before value imports.

```ds organize-imports=on
import { value1, type Type1, value2, type Type2 } from "module"
```

```ds expected
import { type Type1, type Type2, value1, value2 } from "module";
```

### mixed with default

Type imports sorted, then value imports sorted.

```ds organize-imports=on
import Default, { zebra, type Animal, apple, type Fruit } from "module"
```

```ds expected
import Default, { type Animal, type Fruit, apple, zebra } from "module";
```

## exports

### export specifier sorting

Export specifiers are also sorted.

```ds organize-imports=on
export { zebra, apple, mango } from "fruits"
```

```ds expected
export { apple, mango, zebra } from "fruits";
```

### export with type specifiers

Type exports come before value exports.

```ds organize-imports=on
export { value1, type Type1, value2, type Type2 } from "module"
```

```ds expected
export { type Type1, type Type2, value1, value2 } from "module";
```

## aliases

### sort by alias name

When an alias is present, sort by the alias (local name).

```ds organize-imports=on
import { foo as zebra, bar as apple } from "module"
```

```ds expected
import { bar as apple, foo as zebra } from "module";
```
