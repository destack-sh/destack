# Annotation Reflection

## retained annotations

### annotations of returns annotation descriptors

> Retained annotations can be queried through the reflected type handle.

```ds
function label(value: string): string {
    value
}

@label("user")
struct User {
    name: string;
}

const annotations = annotationsOf(Type.of<User>());
annotations satisfies readonly AnnotationDescriptor[];
```

### documentation descriptors are part of retained metadata

> Documentation, when retained, is exposed through descriptors instead of trivia.

```ds
/// A user in the system.
struct User {
    name: string;
}

const symbol = symbolOf(Type.of<User>());
const docs = symbol?.docs;

docs satisfies DocumentationDescriptor | undefined;
```
