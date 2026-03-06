# Document Highlight

## Local Bindings

### Highlight one local value

Document highlights should mark the local occurrences of the selected value.

```ds:main.ds
const [|/*highlight*/value|] = 1;
const next = [|value|] + [|value|];
```

