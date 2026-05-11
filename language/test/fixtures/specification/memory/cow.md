# Cow

## construction

### cow can hold borrowed storage

`Cow` bridges borrowed and owned APIs without choosing ownership up front.

```ds
import { Cow } from "destack:memory/cow";

function view<L: Lifetime>(name: ReadonlyBorrowed<string, L>): Cow<ReadonlyBorrowed<string, L>, ^string> {
    return Cow.borrowed(name);
}
```

### cow can hold owned storage

Owned values use the second `Cow` parameter.

```ds
import { Cow } from "destack:memory/cow";

let name: ^string = "Ada";
let value = Cow.owned(name);

value satisfies Cow<&readonly string, ^string>;
```

## ownership

### cow can become owned

Borrowed values are cloned only when the cow is not already owned.

```ds
import { Cow } from "destack:memory/cow";

declare function name<L: Lifetime>(): Cow<ReadonlyBorrowed<string, L>, ^string>;

let owned = name().intoOwned();

owned satisfies ^string;
```
