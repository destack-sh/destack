# Interface Members

## members

### interface properties declare types

Interface properties declare types without initializers.

```ds
interface Config {
    value: number;
}
```

### interface optional properties are allowed

Optional interface properties use `?`.

```ds
interface Config {
    value?: number;
}
```

### interface readonly properties are allowed

Interface properties can be readonly.

```ds
interface Config {
    readonly value: number;
}
```

### interface properties can use keyword names

Interface properties can use keywords as names.

```ds
interface Config {
    readonly private: boolean;
    readonly static: boolean;
}
```

### interface methods declare signatures

Interface methods declare call signatures without bodies.

```ds
interface Config {
    getValue(name: string): number;
}
```

### interface call signatures are allowed

Interfaces can declare callable signatures.

```ds
interface Fn {
    (value: string): number;
}

declare const fn: Fn;
fn("ok") satisfies number;
```

### interface construct signatures are allowed

Interfaces can declare constructor signatures.

```ds
interface Factory {
    new (value: string): { value: string };
}

declare const Factory: Factory;
const obj = new Factory("ok");
obj satisfies { value: string };
```

### interface accessors are allowed

Interface accessors declare getter and setter signatures.

```ds
interface Config {
    get value(): string;
    set value(value: string);
}
```

### interface index signatures are allowed

Interface index signatures declare dynamic property shapes.

```ds
interface Config {
    [key: string]: number;
}
```

## rejections

### interface members cannot use static modifiers

Interface members cannot be static.

```ds
interface Config {
    static value: number;
}
```

- contains: invalid member modifier

### interface getters cannot take parameters

Interface getters take no parameters.

```ds
interface Config {
    get value(this: Config): string;
}
```

- contains: invalid member modifier

### interface getters cannot be generic

Interface accessors cannot be generic.

```ds
interface Config {
    get value<T>(): T;
}
```

- contains: invalid member modifier

### interface setters require one parameter

Interface setters require one non-optional parameter.

```ds
interface Config {
    set value();
}
```

- contains: invalid member modifier

### interface setters cannot be optional

Interface setters require one required parameter.

```ds
interface Config {
    set value(value?: string);
}
```

- contains: invalid member modifier

### interface setter return type must be void

Interface setters only return void.

```ds
interface Config {
    set value(v: string): string;
}
```

- contains: invalid member modifier
