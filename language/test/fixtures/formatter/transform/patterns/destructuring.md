# Destructuring Patterns

Destructuring pattern fixtures cover nested object and array targets in expression and parameter positions.

## Complex Destructuring

### nested object destructuring

Deeply nested destructuring patterns.

```ds
const { user: { profile: { name, avatar } } } = data
```

```ds expected
const {
    user: {
        profile: { name, avatar },
    },
} = data;
```

### mixed destructuring with defaults

Destructuring with default values and renaming.

```ds line-width=60
const { name = "default", count: total = 0, items: [...rest] } = config
```

```ds expected
const {
    name = "default",
    count: total = 0,
    items: [...rest],
} = config;
```

### array destructuring with rest

Array destructuring with rest patterns.

```ds
const [first, second, ...remaining] = items
```

```ds expected
const [first, second, ...remaining] = items;
```

## Advanced Destructuring

### deeply nested object destructuring

Multiple levels of nested object destructuring.

```ds line-width=60
const { user: { profile: { settings: { theme, language } } } } = config
```

```ds expected
const {
    user: {
        profile: {
            settings: { theme, language },
        },
    },
} = config;
```

### nested destructuring with defaults

Defaults appear at several nesting levels.
The pattern expands when over line width.

```ds line-width=50
const { a: { b = 1, c: { d = 2 } = {} } = {} } = obj
```

```ds expected
const { a: { b = 1, c: { d = 2 } = {} } = {} } =
    obj;
```

### array destructuring with nested objects

Array elements containing object destructuring expand when needed.

```ds line-width=50
const [{ name, id }, { name: secondName }] = items
```

```ds expected
const [{ name, id }, { name: secondName }] =
    items;
```

### mixed array and object destructuring

Complex pattern combining arrays and objects.

```ds line-width=60
const { items: [first, { value: secondValue }, ...rest] } = data
```

```ds expected
const {
    items: [first, { value: secondValue }, ...rest],
} = data;
```

### destructuring in function parameters

Destructuring can appear in arrow function parameters.
The parameter stays hugged while fields break.

```ds line-width=50
const handler = ({ event: { target, type }, timestamp }) => process(target, type)
```

```ds expected
const handler = ({
    event: { target, type },
    timestamp,
}) => process(target, type);
```

### rest in nested destructuring

Rest patterns at different levels.

```ds
const { a, ...rest } = obj
const [first, ...remaining] = arr
```

```ds expected
const { a, ...rest } = obj;
const [first, ...remaining] = arr;
```

### computed property in destructuring

Computed property names stay inside destructuring patterns.

```ds
const { [key]: value, [prefix + suffix]: other } = obj
```

```ds expected
const { [key]: value, [prefix + suffix]: other } = obj;
```

### destructuring with type annotation

Destructuring with TypeScript-style type annotations.

```ds line-width=60
const { name, age }: { name: string, age: number } = person
```

```ds expected
const { name, age }: { name: string; age: number } = person;
```
