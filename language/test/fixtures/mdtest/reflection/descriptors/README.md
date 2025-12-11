# Descriptors

> NOTE #Incomplete: implement/mdtest Type<T> descriptors

Runtime type information via `Type<T>`.

## Coverage

- **Type as value**: Using type name in value position
- **Type<T>**: The descriptor type
- **typeOf()**: Get descriptor for a value
- **Properties**: `.name`, `.fields`, etc.
- **Type guards**: `.is(value)` for runtime checks
- **Factory**: `.create(data)` for instantiation

## Example

```ds
struct User { name: string, age: uint }

// Type in value position
const UserType = User;              // Type<User>
UserType.name                       // "User"
UserType.fields                     // [{ name: "name", type: string }, ...]

// Type guard
if (User.is(value)) {
    // value is User
}

// Factory
const user = User.create({ name: "Alice", age: 30 });
```
