# Data Module Imports

## JSON Import

### import JSON file resolves successfully

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

// data is imported successfully (type inference for JSON not yet implemented)
data;
```

### import JSON array resolves successfully

> Arrays can be imported from JSON files.

```json:numbers.json
[1, 2, 3, 4, 5]
```

```ds:main.ds
import numbers from "./numbers.json";

numbers;
```

## Import Attributes

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

// both imports resolve successfully to different modules
json;
text;
```

### re-export supports type attribute loader overrides

> Re-export declarations apply the same loader override semantics as imports.

```json:data.json
{ "key": "value" }
```

```ds:bridge.ds
export { default as text } from "./data.json" with { type: "text" };
```

```ds:main.ds
import { text } from "./bridge.ds";

text satisfies string;
```

### unknown type attribute reports an error

> Unknown loader type attributes should be rejected.

```json:data.json
{ "key": "value" }
```

```ds:main.ds
import data from "./data.json" with { type: "jsonc" };

data;
```

- contains: invalid import attribute type

## Text Import

### import text file as string

> Text files import successfully.

```text:readme.txt
Hello, World!
```

```ds:main.ds
import content from "./readme.txt";

content;
```

## Type Safety

### property access on imported JSON is typed

> Accessing properties on imported JSON data should have the correct types.

```json:data.json
{ "name": "Alice", "age": 30 }
```

```ds:main.ds
import data from "./data.json";

data.name satisfies string;
data.age satisfies number;
```

### nested object property access is typed

> Nested object properties should be correctly typed.

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

> Array imports should have the correct element type.

```json:numbers.json
[1, 2, 3]
```

```ds:main.ds
import numbers from "./numbers.json";

numbers[0] satisfies number;
```

### text import is typed as string

> Text modules are always typed as string.

```text:readme.txt
Hello, World!
```

```ds:main.ds
import content from "./readme.txt";

content satisfies string;
```

### nonexistent property access is error

> Accessing a property that doesn't exist on imported data should be a type error.

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

### TypeScript namespace import from JSON is typed

> TypeScript namespace JSON imports resolve and expose JSON object members directly.

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
