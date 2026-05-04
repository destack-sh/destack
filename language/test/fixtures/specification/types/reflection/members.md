# Members

Reflection exposes stable descriptors for fields, methods, and signatures.

## fields

### fields of returns field descriptors

Field descriptors expose names and field types.

```ds
struct User {
    id: int64;
    name: string;
}

const fields = fieldsOf(Type.of<User>());
fields satisfies readonly FieldDescriptor[];
```

### field descriptors retain annotations

Retained field annotations are attached to the field descriptor.

```ds
function label(value: string): string {
    value
}

struct User {
    @label("display")
    name: string;
}

const field = fieldsOf(Type.of<User>())[0];
field.annotations satisfies readonly AnnotationDescriptor[];
```

## methods

### methods of returns method descriptors

Method descriptors expose signatures.

```ds
struct User {
    name: string;

    display(): string {
        this.name
    }
}

const methods = methodsOf(Type.of<User>());
methods satisfies readonly MethodDescriptor[];
```

### call signatures expose parameters and returns

Function-like types expose callable signatures.

```ds
type Parser = (raw: string) => number;

const signatures = callSignaturesOf(Type.of<Parser>());
const signature = signatures[0];

signature.parameters satisfies readonly ParameterDescriptor[];
signature.returnType satisfies Type<number>;
```
