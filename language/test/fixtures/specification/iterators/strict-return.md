# Iterator Returns

Tests for `strictBuiltinIteratorReturn`.

## strictBuiltinIteratorReturn

### strict builtin iterator return is undefined

> Builtin iterator return defaults to `undefined` in strict mode.

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "strictBuiltinIteratorReturn": true,
    "lib": ["es5", "es2015.iterable"]
  }
}
```

```ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```

- contains: not assignable

### non-strict builtin iterator return is any

> Builtin iterator return defaults to `any` when strict mode is off.

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noAny": false,
    "strictBuiltinIteratorReturn": false,
    "lib": ["es5", "es2015.iterable"]
  }
}
```

```ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```
