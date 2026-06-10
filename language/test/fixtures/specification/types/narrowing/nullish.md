# Nullish Narrowing

Nullish narrowing via equality guards.

## equality guards

### not equal null narrows to non nullish

`!= null` excludes both nullish values at once.

```ds
const value: string | null | undefined = null;
if (value != null) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### equal null narrows to nullish

`== null` keeps exactly the nullish values.

```ds
const value: string | null | undefined = null;
if (value == null) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### equal undefined narrows to nullish

`== undefined` behaves identically to `== null`.

```ds
const value: string | null | undefined = undefined;
if (value == undefined) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### equal null narrows to nullish with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null == value) {
    value satisfies null | undefined;
} else {
    value satisfies string;
}
```

### not equal null narrows to non nullish with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null != value) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### not equal undefined narrows to non nullish

`!= undefined` excludes both nullish values too.

```ds
const value: string | null | undefined = undefined;
if (value != undefined) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### not equal undefined narrows to non nullish with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = undefined;
if (undefined != value) {
    value satisfies string;
} else {
    value satisfies null | undefined;
}
```

### strict equal null narrows to null

`===` distinguishes the two nullish values.

```ds
const value: string | null | undefined = null;
if (value === null) {
    value satisfies null;
} else {
    value satisfies string | undefined;
}
```

### strict equal null narrows to null with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null === value) {
    value satisfies null;
} else {
    value satisfies string | undefined;
}
```

### strict equal undefined narrows to undefined

`=== undefined` keeps only `undefined`.

```ds
const value: string | null | undefined = undefined;
if (value === undefined) {
    value satisfies undefined;
} else {
    value satisfies string | null;
}
```

### strict equal undefined narrows to undefined with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = undefined;
if (undefined === value) {
    value satisfies undefined;
} else {
    value satisfies string | null;
}
```

### strict not equal undefined narrows to non undefined

`!== undefined` keeps `null` in the union.

```ds
const value: string | null | undefined = undefined;
if (value !== undefined) {
    value satisfies string | null;
} else {
    value satisfies undefined;
}
```

### strict not equal undefined narrows to non undefined with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = undefined;
if (undefined !== value) {
    value satisfies string | null;
} else {
    value satisfies undefined;
}
```

### strict not equal null narrows to non null

`!== null` keeps `undefined` in the union.

```ds
const value: string | null | undefined = null;
if (value !== null) {
    value satisfies string | undefined;
} else {
    value satisfies null;
}
```

### strict not equal null narrows to non null with symbol on right

Comparison direction does not matter.

```ds
const value: string | null | undefined = null;
if (null !== value) {
    value satisfies string | undefined;
} else {
    value satisfies null;
}
```

