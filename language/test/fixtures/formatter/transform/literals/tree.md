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

## JSX Comments

### comment in expression container

Comments inside JSX use expression containers. Block infix comments cause expansion with proper indent.

```ds
<Container>{/* XOXO: something something add content */}</Container>
```

```ds expected
<Container>
    {
        /* XOXO: something something add content */
    }
</Container>;
```

## Complex Expression Children

### map expression in children

Map expressions can generate multiple elements.

```ds line-width=50
<List>{items.map((item) => <Item key={item.id} />)}</List>
```

```ds expected
<List>
    {items.map((item) => <Item key={item.id} />)}
</List>;
```

### long map with block body

Map with block body breaks appropriately. Return JSX gets parens when multi-line.

```ds line-width=40
<List>{items.map((item) => { return <Item key={item.id} name={item.name} /> })}</List>
```

```ds expected
<List>
    {items.map((item) => {
        return (
            <Item
                key={item.id}
                name={item.name}
            />
        )
    })}
</List>;
```

### conditional with jsx

Conditional expressions with JSX children.

```ds
<div>{loading && <Spinner />}</div>
```

```ds expected
<div>{loading && <Spinner />}</div>;
```

### ternary with complex jsx branches

Ternary with multi-attribute JSX in branches.

```ds line-width=50
<div>{loading ? <Spinner size="large" /> : <Content data={data} />}</div>
```

```ds expected
<div>
    {loading
        ? <Spinner size="large" />
        : <Content data={data} />}
</div>;
```

## Multiple Callbacks

### element with multiple callback props

Multiple callbacks break element to multi-line.

```ds line-width=50
<Button onClick={handleClick} onHover={handleHover} onFocus={handleFocus} />
```

```ds expected
<Button
    onClick={handleClick}
    onHover={handleHover}
    onFocus={handleFocus}
/>;
```

### callback with inline arrow function

Inline arrow functions as callbacks.

```ds line-width=60
<Button onClick={() => setOpen(true)} onClose={() => setOpen(false)} />
```

```ds expected
<Button
    onClick={() => setOpen(true)}
    onClose={() => setOpen(false)}
/>;
```

## JSX as Arguments

### jsx in function call

JSX can be passed as function argument.

```ds
render(<App />)
```

```ds expected
render(<App />);
```

### jsx with props in function call

JSX with attributes in function arguments. Boolean `{true}` uses shorthand.

```ds
createPortal(<Modal isOpen={true} />, document.body)
```

```ds expected
createPortal(<Modal isOpen />, document.body);
```

### complex jsx in function call breaks

Complex JSX in function call breaks to new line.

```ds line-width=40
render(<Container><Header /><Content /></Container>)
```

```ds expected
render(
    <Container>
        <Header />
        <Content />
    </Container>,
);
```

## Return Statements with JSX

### return jsx inline

Short JSX in return statements stays inline.

```ds
return <App />
```

```ds expected
return <App />;
```

### return jsx multiline gets wrapped

When JSX in return breaks, it gets wrapped in parentheses.

```ds line-width=30
return <App prop="value" another="thing" />
```

```ds expected
return (
    <App
        prop="value"
        another="thing"
    />
);
```

### return nested jsx wrapped

Nested JSX in returns also gets wrapped.

```ds line-width=40
return <Container><Header /><Content /></Container>
```

```ds expected
return (
    <Container>
        <Header />
        <Content />
    </Container>
);
```
