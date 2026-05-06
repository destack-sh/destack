# Character

## values

### character literals satisfy character

Character literals are assignable to the character type.

```ds
let value: character = 'a';
```

### character does not widen to string

Character values do not implicitly widen to string.

```ds
let value: string = 'a';
```

- contains: not assignable

### string literals do not satisfy character

String literals are not assignable to character.

```ds
let value: character = "a";
```

- contains: not assignable

### character values are not assignable to numbers

Character values do not implicitly convert to numeric types.

```ds
let value: int32 = 'a';
```

- contains: not assignable

### character values satisfy character unions

Character values are assignable to unions that include character.

```ds
let value: character | string = 'a';
value satisfies character | string;
```
