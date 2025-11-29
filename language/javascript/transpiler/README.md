# javascript/transpiler

Destack to JavaScript transpiler.
Transforms DIR into JavaScript/TypeScript source code.

## Layout

```
src/
├── transpile/   DIR → JS AST transformation
├── format/      JS AST → source text formatting
├── diagnostic/  Transpiler errors and warnings
└── tests/       Transpiler tests
```

