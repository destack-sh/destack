# Import Specifier Sorting

Import specifier fixtures cover sorting inside import and export braces.

## Specifier Sorting

### alphabetical order

Import specifiers should be sorted alphabetically.

```tspp
import { zebra, apple, mango } from "fruits"
```

```tspp expected
import { apple, mango, zebra } from "fruits";
```

### with default import

Default import stays first, remaining specifiers are sorted.

```tspp
import Default, { zebra, apple, mango } from "fruits"
```

```tspp expected
import Default, { apple, mango, zebra } from "fruits";
```

### natural sort order

Numbers are sorted as integers, not lexicographically.

```tspp
import { item10, item2, item1 } from "items"
```

```tspp expected
import { item1, item2, item10 } from "items";
```

## exports

### export specifier sorting

Export specifiers are also sorted.

```tspp
export { zebra, apple, mango } from "fruits"
```

```tspp expected
export { apple, mango, zebra } from "fruits";
```

## aliases

### sort by alias name

When an alias is present, sort by the alias (local name).

```tspp
import { foo as zebra, bar as apple } from "module"
```

```tspp expected
import { bar as apple, foo as zebra } from "module";
```
