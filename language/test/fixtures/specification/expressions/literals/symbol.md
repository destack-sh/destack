# Symbol Literals

Symbol values and member access.

## symbols

### symbol constructor

> Symbols can be created via Symbol().


```ds libs=es2015
const x: symbol = Symbol("id");
```

## Symbol Members

### symbol toString resolves

> Symbols expose Symbol standard members.

```ds libs=es2015
const value: symbol = Symbol("id");
const text = value.toString();
text satisfies string;
```

### symbols are not assignable to strings

> Symbol values are not assignable to string.

```ds libs=es2015
const value: string = Symbol("id");
```

- type symbol is not assignable to type number

### symbol values compose with symbol unions

> Symbol values can flow into unions that include symbol.

```ds libs=es2015
const value: symbol | string = Symbol("id");
value satisfies symbol | string;
```

### symbol values reject numeric annotations

> Symbol values are not assignable to number.

```ds libs=es2015
const value: number = Symbol("id");
```

- type symbol is not assignable to type number
