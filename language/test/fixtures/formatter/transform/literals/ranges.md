# Range Literals

Tests for Destack range literal formatting.

## Exclusive Ranges

### exclusive range

Range literals use `..` with no surrounding spaces.

```ds
0..10
```

```ds expected
0..10;
```

### range with expressions

Spaces around `..` should be removed.

```ds
start .. end
```

```ds expected
start..end;
```

## For Loops with Ranges

### for-of with range

Ranges are commonly used in for-of loops.

```ds
for ( const i of 0 .. 10 ) { }
```

```ds expected
for (const i of 0..10) { }
```
