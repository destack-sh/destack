# Tree Attribute Values

## Elements with Attributes

### tree element with attributes

Non-string attribute values use braces with normalized spacing.

```tspp
<Entity  a = {1}  b = {2}  />
```

```tspp expected
<Entity a={1} b={2} />;
```

### attributes with expressions

Attribute values can be expressions in braces.

```tspp
<Button onClick={handleClick} disabled={isLoading} />
```

```tspp expected
<Button onClick={handleClick} disabled={isLoading} />;
```

### attribute with fixed array value

Fixed array repeat literals can be passed as attribute values.

```tspp
<Buffer data={[0; count]} />
```

```tspp expected
<Buffer data={[0; count]} />;
```


## Attribute Values

### attribute with object value breaks with element

When an attribute value doesn't fit, the whole element breaks to multi-line format.

```tspp line-width=30
<Button style={{ color: "red", fontSize: 14 }} />
```

```tspp expected
<Button
    style={{
        color: "red",
        fontSize: 14,
    }}
/>;
```

### boolean attribute without value

Boolean attributes can omit the value.

```tspp
<Input disabled readonly />
```

```tspp expected
<Input disabled readonly />;
```
