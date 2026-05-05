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

Imported JSON properties keep literal types.

```json:data.json
{ "name": "Alice", "age": 30 }
```

```ds:main.ds
import data from "./data.json";

data.name satisfies "Alice";
data.age satisfies 30;
```

### nested object property access is typed

> Nested object properties keep literal types.

```json:config.json
{
    "server": { "host": "localhost", "port": 8080 },
    "debug": true
}
```

```ds:main.ds
import config from "./config.json";

config.server.host satisfies "localhost";
config.server.port satisfies 8080;
config.debug satisfies true;
```

### array imports are tuples

> Array imports keep literal tuple shape.

```json:numbers.json
[1, 2, 3]
```

```ds:main.ds
import numbers from "./numbers.json";

numbers satisfies readonly [1, 2, 3];
numbers[0] satisfies 1;
numbers[2] satisfies 3;
```

### imported JSON can be widened

Imported JSON can flow into a broader annotated type.

```json:data.json
{ "name": "Alice", "age": 30 }
```

```ds:main.ds
interface User {
    name: string;
    age: number;
}

import data from "./data.json";

const user: User = data;
user.name satisfies string;
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
