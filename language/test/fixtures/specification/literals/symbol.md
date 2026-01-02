# Symbol Literals

Tests for symbol values and member access.

## Basic Symbols

### symbol constructor

> Symbols can be created via Symbol().


```ds libs=es2015
const x: symbol = Symbol("id");
```

## Symbol Members

### symbol toString resolves

> Symbols expose Symbol prototype members.


```ds libs=es2015
const value: symbol = Symbol("id");
const text = value.toString();
text satisfies string;
```
