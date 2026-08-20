# Class Decorators

## Decorators

### decorated class

Decorators appear on their own line before the class.

```ds
@Component
class MyComponent { }
```

```ds expected
@Component
class MyComponent {}
```

### decorator with arguments

Decorator arguments follow function call formatting.

```ds
@Component({ selector: "my-component" })
class MyComponent { }
```

```ds expected
@Component({ selector: "my-component" })
class MyComponent {}
```

### multiple decorators

Decorator calls keep their empty parentheses.

```ds
@Injectable()
@Singleton
class Service { }
```

```ds expected
@Injectable()
@Singleton
class Service {}
```

### decorator expressions with calls

Complex decorator expressions use parentheses for clarity.

```ds
@factory().decorator
@factory().decorator()
@decorator().member
@decorator().member()
class Service { }
```

```ds expected
@(factory().decorator)
@(factory().decorator())
@(decorator().member)
@(decorator().member())
class Service {}
```

### decorator instantiation expressions

Decorator instantiation expressions use parentheses.

```ds
@decorator<T>
class Service { }
```

```ds expected
@(decorator<T>)
class Service {}
```

### decorated field

Field decorators appear on their own line above the field.

```ds
class Foo { @observable x: number }
```

```ds expected
class Foo {
    @observable
    x: number;
}
```

### decorated method

Method decorators appear on their own line above the method.

```ds
class Foo { @memoize compute(): number { return 42 } }
```

```ds expected
class Foo {
    @memoize
    compute(): number {
        return 42;
    }
}
```
