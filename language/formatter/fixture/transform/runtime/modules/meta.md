# Import Meta

Import meta fixtures cover `import.meta` member chains.

## Import Meta

### import.meta property access

Import meta property access keeps dots tight.

```tspp:main.tspp
const url = import.meta.url
```

```tspp expected
const url = import.meta.url;
```

### import.meta nested property access

Nested import meta property access stays as a normal chain.

```tspp:main.tspp
const mode = import.meta.env.MODE
```

```tspp expected
const mode = import.meta.env.MODE;
```
