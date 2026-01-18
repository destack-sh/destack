# Try Catch Variables

## useUnknownInCatchVariables

### useUnknownInCatchVariables true uses unknown

> Catch variables default to unknown when enabled.

```ds:dsconfig.json
{ "compilerOptions": { "useUnknownInCatchVariables": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```

- contains: expected string

### useUnknownInCatchVariables false uses any

> Catch variables default to any when disabled.

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false, "useUnknownInCatchVariables": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```
