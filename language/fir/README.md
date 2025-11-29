# fir

Destack FIR (Formatting Intermediate Representation).
A document model for pretty-printing code with configurable line width and indentation.

## Layout

```
src/
├── format/      Document building (groups, labels, spacing)
└── print/       Document printing and line fitting
```

---

Based on [Ruff's formatter IR](https://github.com/astral-sh/ruff) (MIT).

