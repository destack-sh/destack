# Character Literals

## character literals

### character literal yields character type

> Character literals infer to the character type.

```ds
const value = 'a';
value satisfies character;
```

### character literal does not widen to string

> Character literals do not implicitly widen to string.

```ds
const value: character = 'a';
const text: string = value;
```

- contains: not assignable
