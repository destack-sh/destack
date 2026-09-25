# Destructuring Patterns

Destructuring pattern fixtures cover nested object and array targets in expression and parameter positions.

## Nested Destructuring

### nested object destructuring

Deeply nested destructuring patterns.

```tspp
const { user: { profile: { name, avatar } } } = data
```

```tspp expected
const {
    user: {
        profile: { name, avatar },
    },
} = data;
```

### mixed destructuring with defaults

Destructuring with default values and renaming.

```tspp line-width=60
const { name = "default", count: total = 0, items: [...rest] } = config
```

```tspp expected
const {
    name = "default",
    count: total = 0,
    items: [...rest],
} = config;
```

### array destructuring with rest

Array destructuring with rest patterns.

```tspp
const [first, second, ...remaining] = items
```

```tspp expected
const [first, second, ...remaining] = items;
```

## Default Destructuring

### deeply nested object destructuring

Multiple levels of nested object destructuring.

```tspp line-width=60
const { user: { profile: { settings: { theme, language } } } } = config
```

```tspp expected
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

```tspp line-width=50
const { a: { b = 1, c: { d = 2 } = {} } = {} } = obj
```

```tspp expected
const { a: { b = 1, c: { d = 2 } = {} } = {} } =
    obj;
```

### array destructuring with nested objects

Array elements containing object destructuring expand when needed.

```tspp line-width=50
const [{ name, id }, { name: secondName }] = items
```

```tspp expected
const [{ name, id }, { name: secondName }] =
    items;
```

### mixed array and object destructuring

Complex pattern combining arrays and objects.

```tspp line-width=60
const { items: [first, { value: secondValue }, ...rest] } = data
```

```tspp expected
const {
    items: [first, { value: secondValue }, ...rest],
} = data;
```

### destructuring in function parameters

Destructuring can appear in arrow function parameters.
The parameter stays hugged while fields break.

```tspp line-width=50
const handler = ({ event: { target, type }, timestamp }) => process(target, type)
```

```tspp expected
const handler = ({
    event: { target, type },
    timestamp,
}) => process(target, type);
```

### rest in nested destructuring

Rest patterns at different levels.

```tspp
const { a, ...rest } = obj
const [first, ...remaining] = arr
```

```tspp expected
const { a, ...rest } = obj;
const [first, ...remaining] = arr;
```

### computed property in destructuring

Computed property names stay inside destructuring patterns.

```tspp
const { [key]: value, [prefix + suffix]: other } = obj
```

```tspp expected
const { [key]: value, [prefix + suffix]: other } = obj;
```

### destructuring with type annotation

Destructuring patterns keep type annotations.

```tspp line-width=60
const { name, age }: { name: string, age: number } = person
```

```tspp expected
const { name, age }: { name: string; age: number } = person;
```
