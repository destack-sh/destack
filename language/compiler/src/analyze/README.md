# Analyze

Analyze is where Destack commits program meaning on top of Resolve-identified symbol bindings.
Analyze commits declaration meaning, expression meaning, module boundary meaning, and semantic legality.
If a semantic commitment is missing after Analyze, that is an Analyze bug.

## Pipeline

Analyze runs after Resolve and before Elaborate.

```text
Import -> Resolve -> Analyze -> Elaborate -> Execute -> Lower
```

Analyze is staged and ordered.

```text
Declare -> Interface -> Infer -> Solve -> Commit -> Validate
```

### Input

Analyze input is resolved DIR with symbol bindings and module dependency structure.
All `Unresolved*` forms should already be eliminated before Analyze.
`unknown` remains a real type value inside Analyze.

### Output

Analyze output is semantically committed DIR plus diagnostics.
Downstream phases should see stable declaration and expression meaning.
