# Data

## JSON import

### JSON object imports resolve

> JSON files can be imported as data modules.

```json:data.json
{
    "name": "Alice",
    "age": 30,
    "active": true
}
```

```ds:main.ds
import data from "./data.json";

data;
```

### JSON array imports resolve

> Arrays can be imported from JSON files.

```json:numbers.json
[1, 2, 3, 4, 5]
```

```ds:main.ds
import numbers from "./numbers.json";

numbers;
```

## import attributes

### override loader with type attribute

> The `type` attribute overrides the default loader for a file.

```json:data.json
{ "key": "value" }
```

```ds:main.ds
import text from "./data.json" with { type: "text" };

// imported as text (string), not as JSON object
text;
```

### same file with different loaders produces different modules

> The same file can be imported multiple times with different loaders.

```json:data.json
{ "key": "value" }
```

```ds:main.ds
import json from "./data.json";
import text from "./data.json" with { type: "text" };

json;
text;
```

### re-export supports type attribute loader overrides

> Re-export declarations apply the same loader override semantics as imports.

```json:data.json
{ "key": "value" }
```

```ds:forward.ds
export { default as text } from "./data.json" with { type: "text" };
```

```ds:main.ds
import { text } from "./forward.ds";

text satisfies string;
```

### unknown type attribute reports an error

> Unknown loader type attributes are rejected.

```json:data.json
{ "key": "value" }
```

```ds:main.ds
import data from "./data.json" with { type: "jsonc" };

data;
```

- contains: invalid import attribute type

## typing

### property access on imported JSON is typed

Imported JSON properties expose their inferred types.

```json:data.json
{ "name": "Alice", "age": 30 }
```

```ds:main.ds
import data from "./data.json";

data.name satisfies string;
data.age satisfies number;
```

### nested object property access is typed

> Nested object properties are typed.

```json:config.json
{
    "server": { "host": "localhost", "port": 8080 },
    "debug": true
}
```

```ds:main.ds
import config from "./config.json";

config.server.host satisfies string;
config.server.port satisfies number;
config.debug satisfies boolean;
```

### array import has element type

> Array imports expose their element type.

```json:numbers.json
[1, 2, 3]
```

```ds:main.ds
import numbers from "./numbers.json";

numbers[0] satisfies number;
```

### nonexistent property access is error

> Accessing a property that doesn't exist on imported data is a type error.

```json:data.json
{ "name": "Alice" }
```

```ds:main.ds
import data from "./data.json";

data.nonexistent;
```

- contains: property

### base64 import is typed as string

> Files imported with base64 loader are encoded and typed as string.

```text:data.txt
Hello!
```

```ds:main.ds
import encoded from "./data.txt" with { type: "base64" };

encoded satisfies string;
```

### namespace import from JSON is typed

> Namespace JSON imports resolve and expose JSON object members directly.

```json:data.json
{
    "name": "Alice",
    "age": 30
}
```

```ts:main.ts
import * as data from "./data.json";

data.name satisfies string;
data.age satisfies number;
```
