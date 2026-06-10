# Range Precedence

Range expressions keep arithmetic inside their endpoints.

## endpoints

### negative starts bind as prefix expressions

Prefix minus binds tighter than `..`.

```ds
const range = -3..3;

range satisfies Range<int>;
```

### arithmetic binds inside endpoints

Binary arithmetic binds tighter than `..`.

```ds
const start = 1;
const end = 4;
const range = start + 1..end * 2;

range satisfies Range<int>;
```

### newline ends an open-ended range

An open end stops at the line break.

```ds
const from = 1..;
const to = ..10;

from satisfies RangeFrom<int>;
to satisfies RangeTo<int>;
```
