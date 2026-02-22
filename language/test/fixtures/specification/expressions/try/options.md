# Try Options

## noExceptions

### noExceptions forbids throw

> Throw expressions are rejected when exceptions are disabled.

```json:dsconfig.json
{ "compilerOptions": { "noExceptions": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
function fail(): never {
    throw "error";
}
```

- contains: exceptions are disabled

### noExceptions allows try when no throw occurs

> Try expressions remain valid when exceptions are disabled, as long as no throw is used.

```json:dsconfig.json
{ "compilerOptions": { "noExceptions": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch (_error) {
    0
};

value satisfies int;
```

### noExceptions false allows throw

> Throw expressions are allowed when exceptions are enabled.

```json:dsconfig.json
{ "compilerOptions": { "noExceptions": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
function fail(): never {
    throw "error";
}
```

### noExceptions forbids throw in try bodies

> Throw remains forbidden inside try blocks when exceptions are disabled.

```json:dsconfig.json
{ "compilerOptions": { "noExceptions": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    throw "error";
} catch (_error) {
    0
};

value satisfies int32;
```

- contains: exceptions are disabled

### noExceptions forbids throw in catch bodies

> Throw remains forbidden inside catch blocks when exceptions are disabled.

```json:dsconfig.json
{ "compilerOptions": { "noExceptions": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch (_error) {
    throw "error";
};

value satisfies int32;
```

- contains: exceptions are disabled
