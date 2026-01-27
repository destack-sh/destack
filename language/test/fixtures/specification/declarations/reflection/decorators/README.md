# Decorators

> NOTE #Incomplete: implement/mdtest decorator metadata

Runtime access to decorator information.

## Coverage

- **Accessing decorators**: `.decorators` property
- **Decorator shape**: `{ name, arguments }`
- **Multiple decorators**: Array of decorator info

## Example

```ds
@deprecated("use newAPI")
@version(2)
function oldAPI() { }

oldAPI.decorators
// [
//   { name: "deprecated", arguments: ["use newAPI"] },
//   { name: "version", arguments: [2] }
// ]
```
