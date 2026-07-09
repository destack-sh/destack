# Import Meta

Import meta fixtures cover `import.meta` member chains.

## Import Meta

### import.meta property access

Import meta property access keeps dots tight.

```ds:main.ds
const url = import.meta.url
```

```ds expected
const url = import.meta.url;
```

### import.meta nested property access

Nested import meta property access stays as a normal chain.

```ds:main.ds
const mode = import.meta.env.MODE
```

```ds expected
const mode = import.meta.env.MODE;
```
