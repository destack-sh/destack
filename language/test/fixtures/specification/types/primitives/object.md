# Object Type

The `object` type, which represents any non-primitive value.

## object

### object to object

> The object type is assignable to itself.

```ds
const x: object = {} as object
```

### object literal to object

> Object literals are assignable to object.

```ds
const x: object = { a: 1, b: "hello" }
```

### array to object

> Arrays are assignable to object.

```ds
const x: object = [1, 2, 3]
```

### function to object

> Functions are assignable to object.

```ds
const fn = () => {}
const x: object = fn
```

## Object Rejects Primitives

### number not assignable to object

> Primitive number is not assignable to object.

```ds
const x: object = 42
```

- type undefined is not assignable to type object

### string not assignable to object

> Primitive string is not assignable to object.

```ds
const x: object = "hello"
```

- type undefined is not assignable to type object

### boolean not assignable to object

> Primitive boolean is not assignable to object.

```ds
const x: object = true
```

- type undefined is not assignable to type object

### null not assignable to object

> Null is not assignable to object.

```ds
const x: object = null
```

- type undefined is not assignable to type object

### undefined not assignable to object

> Undefined is not assignable to object.

```ds
const x: object = undefined
```

- type undefined is not assignable to type object
