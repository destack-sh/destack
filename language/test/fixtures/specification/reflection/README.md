# Reflection (LanguageFeature::Reflection)

> NOTE #Incomplete: implement/mdtest runtime reflection

Types as first-class runtime values.

In TypeScript, types are erased at runtime. Destack makes types first-class runtime
values, enabling reflection without separate metadata systems.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `descriptors/` | `Type<T>`, `.name`, `.fields`, `.is()` |
| `decorators/` | Runtime access to decorator info |

## Example

```ds
struct User { name: string, age: uint }

// User in type position: the type
let u: User = User { name: "Alice", age: 30 };

// User in value position: the type descriptor
const UserType = User;              // UserType: Type<User>
UserType.name                       // "User"
UserType.fields                     // [{ name: "name", type: string }, ...]

if (User.is(value)) {
    // value is User
}
```

See [DESIGN.md](../../../../../DESIGN.md#reflection) for full documentation.
