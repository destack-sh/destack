# Reflection Shapes

`Type.describe` exposes source-facing type shapes.

## references

### reference shapes expose declarations

```ds
struct User {
    name: string;
}

const shape = Type.describe(Type.of<User>()).shape;

if (shape.kind == "reference") {
    shape satisfies ReferenceType;
    shape.declaration.name satisfies string;
}
```

## sequences

### fixed arrays expose element and count

```ds
const shape = Type.describe(Type.of<[uint8; 4]>()).shape;

if (shape.kind == "fixedArray") {
    shape satisfies FixedArrayType;
    shape.element satisfies Type<uint8>;
}
```

### slices expose element types

```ds
const shape = Type.describe(Type.of<[uint8]>()).shape;

if (shape.kind == "slice") {
    shape satisfies SliceType;
    shape.element satisfies Type<uint8>;
}
```

### tuples expose ordered elements

```ds
const shape = Type.describe(Type.of<(string, int32)>()).shape;

if (shape.kind == "tuple") {
    shape satisfies TupleType;
    shape.elements satisfies readonly TupleElement[];
}
```

## compounds

### object shapes expose fields

```ds
const shape = Type.describe(Type.of<{ name: string; age?: uint }>()).shape;

if (shape.kind == "object") {
    shape satisfies ObjectType;
    shape.fields satisfies readonly ObjectField[];
}
```

### function shapes expose parameters and returns

```ds
type Parser = (raw: string) => int32;

const shape = Type.describe(Type.of<Parser>()).shape;

if (shape.kind == "function") {
    shape satisfies FunctionType;
    shape.parameters satisfies readonly FunctionParameter[];
    shape.returnType satisfies Type<int32>;
}
```

### union shapes expose members

```ds
const shape = Type.describe(Type.of<string | int32>()).shape;

if (shape.kind == "union") {
    shape satisfies UnionType;
    shape.elements satisfies readonly Type<unknown>[];
}
```
