# Class Decorators

## Decorators

### decorated class

Decorators appear on their own line before the class.

```tspp
@Component
class MyComponent { }
```

```tspp expected
@Component
class MyComponent {}
```

### decorator with arguments

Decorator arguments follow function call formatting.

```tspp
@Component({ selector: "my-component" })
class MyComponent { }
```

```tspp expected
@Component({ selector: "my-component" })
class MyComponent {}
```

### multiple decorators

Decorator calls keep their empty parentheses.

```tspp
@Injectable()
@Singleton
class Service { }
```

```tspp expected
@Injectable()
@Singleton
class Service {}
```

### decorator expressions with calls

Complex decorator expressions use parentheses for clarity.

```tspp
@factory().decorator
@factory().decorator()
@decorator().member
@decorator().member()
class Service { }
```

```tspp expected
@(factory().decorator)
@(factory().decorator())
@(decorator().member)
@(decorator().member())
class Service {}
```

### decorator instantiation expressions

Decorator instantiation expressions use parentheses.

```tspp
@decorator<T>
class Service { }
```

```tspp expected
@(decorator<T>)
class Service {}
```

### decorated field

Field decorators appear on their own line above the field.

```tspp
class Foo { @observable x: number }
```

```tspp expected
class Foo {
    @observable
    x: number;
}
```

### decorated method

Method decorators appear on their own line above the method.

```tspp
class Foo { @memoize compute(): number { return 42 } }
```

```tspp expected
class Foo {
    @memoize
    compute(): number {
        return 42;
    }
}
```
