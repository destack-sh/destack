# Static If Annotations

Multiple `@if` guards must all be true.

## conjunctions

### multiple `@if` annotations combine

Both guards must be true for the declaration to exist.

```ds
@if(import.meta.output == "js")
@if(import.meta.output == "native")
const combined = missingSymbol;
```
