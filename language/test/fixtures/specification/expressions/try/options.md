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
