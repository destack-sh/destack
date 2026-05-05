# Scalars

`number`, `string`, and `boolean` keep their strict scalar behavior.

## number

### number accepts numeric literals

```ds
const value: number = 42;
value satisfies number;
```

### numeric literals can infer number

```ds
const value = 42;
value satisfies number;
```

## string

### string accepts string literals

```ds
const value: string = "hello";
value satisfies string;
```

### string literals infer literal strings

```ds
const value = "hello";
value satisfies "hello";
value satisfies string;
```

## boolean

### boolean accepts boolean literals

```ds
const value: boolean = true;
value satisfies boolean;
```

### boolean literals infer literal booleans

```ds
const value = true;
value satisfies true;
value satisfies boolean;
```
