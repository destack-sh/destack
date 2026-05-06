# Character Literals

## character literals

### character literal yields character type

Character literals infer to the character type.

```ds
const value = 'a';
value satisfies character;
```

### character literal does not widen to string

Character literals do not implicitly widen to string.

```ds
const value: character = 'a';
const text: string = value;
```

- contains: not assignable

### character literals are not assignable to integers

Character literals are not implicitly assignable to integer types.

```ds
const value: int32 = 'a';
```

- contains: not assignable

### character literals work with character unions

Character literals flow into unions that include character.

```ds
const value: character | string = 'a';
value satisfies character | string;
```

### character literals reject multi-character payloads

Character literals require exactly one character payload.

```ds
const value: character = 'ab';
```

- contains: character
