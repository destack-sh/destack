# Modes

## source selection

### active mode files share the base module

Importing the base module includes active mode files.

```ds:user.ds
export const base = "user";
```

```ds:user.test.ds
export const testName = `${base}.test`;
```

```ds:main.ds
import { base, testName } from "./user";

base satisfies "user";
testName satisfies "user.test";
```

```json:destack.json
{ "compiler": { "modes": ["test"] } }
```

### inactive mode files are ignored

Mode files only contribute when their mode is active.

```ds:user.ds
export const base = "user";
```

```ds:user.dev.ds
export const debugName = `${base}.dev`;
```

```ds:main.ds
import { base } from "./user";

base satisfies "user";
```

## metadata

### import meta exposes active modes

`import.meta.modes` is available as a static term.

```ds
const modes = import.meta.modes;
modes satisfies readonly string[];
```

```json:destack.json
{ "compiler": { "modes": ["test", "lint"] } }
```

### builtin mode shorthands follow modes

Mode shorthand booleans are derived from `import.meta.modes`.

```ds
import.meta.modes satisfies readonly string[];
import.meta.debug satisfies boolean;
import.meta.dev satisfies boolean;
import.meta.prod satisfies boolean;
import.meta.test satisfies boolean;
import.meta.bench satisfies boolean;
import.meta.lint satisfies boolean;
```

```json:destack.json
{ "compiler": { "modes": ["debug", "test"] } }
```
