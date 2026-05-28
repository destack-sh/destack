# Symbol Literals

Symbol values and member access.

## symbols

### symbol create

Symbols can be created via `Symbol.create`.


```ds
const x: symbol = Symbol.create("id");
```

## symbol members

### symbol toString resolves

Symbols expose Symbol standard members.

```ds
const value: symbol = Symbol.create("id");
const text = value.toString();
text satisfies string;
```

### symbols are not assignable to strings

Symbol values are not assignable to string.

```ds
const value: string = Symbol.create("id");
```

- contains: not assignable

### symbol values work with symbol unions

Symbol values can flow into unions that include symbol.

```ds
const value: symbol | string = Symbol.create("id");
value satisfies symbol | string;
```

### symbol values reject numeric annotations

Symbol values are not assignable to number.

```ds
const value: number = Symbol.create("id");
```

- contains: not assignable
