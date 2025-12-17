# Tree Literals

Tests for Destack tree literal (JSX-like) formatting.

## Self-Closing Elements

### self-closing tree element

Self-closing elements have a space before `/>`.

```ds
<Entity  />
```

```ds expected
<Entity />;
```

### self-closing with many attributes breaks

When attributes exceed line width, they break to multiple lines.

```ds line-width=30
<Button variant="primary" size="large" disabled />
```

```ds expected
<Button
    variant="primary"
    size="large"
    disabled
/>;
```

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

## Elements with Children

### element with text child

Elements can contain text content.

```ds
<Text>Hello World</Text>
```

```ds expected
<Text>Hello World</Text>;
```

### element with element children

Elements can contain other elements.

```ds
<Container><Header /><Content /></Container>
```

```ds expected
<Container>
    <Header />
    <Content />
</Container>;
```

### nested elements

Deeply nested elements format with proper indentation.

```ds line-width=40
<Outer><Middle><Inner>content</Inner></Middle></Outer>
```

```ds expected
<Outer>
    <Middle>
        <Inner>content</Inner>
    </Middle>
</Outer>;
```

### mixed children

Elements can have mixed content types. Content stays on one line if it fits.

```ds
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>
```

```ds expected
<Paragraph>Hello <Strong>World</Strong> !</Paragraph>;
```

## Complex Patterns

### conditional rendering

Ternary expressions work inside tree elements.

```ds
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>
```

```ds expected
<Container>{isOpen ? <Panel /> : <Placeholder />}</Container>;
```

### spread attributes

Spread operator passes all properties from an object.

```ds
<Button {...props} extra="value" />
```

```ds expected
<Button {...props} extra="value" />;
```

### component with callback

Callbacks can be passed as attributes.

```ds
<Button onClick={(e) => handleClick(e)} />
```

```ds expected
<Button onClick={(e) => handleClick(e)} />;
```

### fragments

Fragment syntax groups elements without a wrapper.

```ds
<><Header /><Content /><Footer /></>
```

```ds expected
<>
    <Header />
    <Content />
    <Footer />
</>;
```

## Hugging

### attribute with object value hugs

When an object attribute value expands, the braces should hug.

```ds line-width=30
<Button style={{ color: "red", fontSize: 14 }} />
```

```ds expected
<Button style={{
    color: "red",
    fontSize: 14,
}} />;
```
