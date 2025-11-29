# dir

Destack DIR (Document Intermediate Representation) and program model.
Represents compiled programs with resolved types, symbols, and module structure.

## Layout

```
src/
├── program/     Program, module, and package definitions
├── tree/        DIR node definitions
├── symbol/      Symbol tables and scopes
├── type/        Type definitions and tables
├── analyze/     Analysis tables
├── formatter/   DIR formatting
└── dump/        DIR dumping utilities
```

