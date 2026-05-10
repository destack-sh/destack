# Tree Attribute Values

## Elements with Attributes

### tree element with attributes

JSX-compliant: non-string values need braces, spacing is normalized.

```ds
<Entity  a = {1}  b = {2}  />
```

```ds expected
<Entity a={1} b={2} />;
```

### attributes with expressions

Attribute values can be expressions in braces.

```ds
<Button onClick={handleClick} disabled={isLoading} />
```

```ds expected
<Button onClick={handleClick} disabled={isLoading} />;
```

### attribute with fixed array value

Fixed array repeat literals can be passed as attribute values.

```ds
<Buffer data=[0; count] />
```

```ds expected
<Buffer data=[0; count] />;
```


## Attribute Values

### attribute with object value breaks with element

When an attribute value doesn't fit, the whole element breaks to multi-line format.

```ds line-width=30
<Button style={{ color: "red", fontSize: 14 }} />
```

```ds expected
<Button
    style={{
        color: "red",
        fontSize: 14,
    }}
/>;
```

### boolean attribute without value

Boolean attributes can omit the value.

```ds
<Input disabled readonly />
```

```ds expected
<Input disabled readonly />;
```
