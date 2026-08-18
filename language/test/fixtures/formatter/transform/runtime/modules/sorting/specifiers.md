# Import Specifier Sorting

Import specifier fixtures cover sorting inside import and export braces.

## Specifier Sorting

### alphabetical order

Import specifiers should be sorted alphabetically.

```ds
import { zebra, apple, mango } from "fruits"
```

```ds expected
import { apple, mango, zebra } from "fruits";
```

### with default import

Default import stays first, remaining specifiers are sorted.

```ds
import Default, { zebra, apple, mango } from "fruits"
```

```ds expected
import Default, { apple, mango, zebra } from "fruits";
```

### natural sort order

Numbers are sorted as integers, not lexicographically.

```ds
import { item10, item2, item1 } from "items"
```

```ds expected
import { item1, item2, item10 } from "items";
```

## exports

### export specifier sorting

Export specifiers are also sorted.

```ds
export { zebra, apple, mango } from "fruits"
```

```ds expected
export { apple, mango, zebra } from "fruits";
```

## aliases

### sort by alias name

When an alias is present, sort by the alias (local name).

```ds
import { foo as zebra, bar as apple } from "module"
```

```ds expected
import { bar as apple, foo as zebra } from "module";
```
