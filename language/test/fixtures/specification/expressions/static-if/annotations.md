# Static If Annotations

Multiple `@if` guards must all be true.

## conjunctions

### multiple `@if` annotations combine

Both guards must be true for the declaration to exist.

```ds
@if(import.meta.runtime == "js")
@if(import.meta.runtime == "destack")
const combined = missingSymbol;
```
