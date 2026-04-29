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

### text whitespace normalizes

Text nodes collapse whitespace to single spaces.

```ds
<Text>  Hello   World </Text>
```

```ds expected
<Text> Hello World </Text>;
```

### whitespace-only text is preserved as a single space

Whitespace-only text normalizes to a single preserved space.

```ds
<Text> </Text>
```

```ds expected
<Text> </Text>;
```

### whitespace expression container normalizes

Whitespace expression containers normalize to inline text spacing.

```ds
<Text>{" "}Hello{" "}World{" "}</Text>
```

```ds expected
<Text> Hello World </Text>;
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

Elements with mixed text and element children expand into a readable multiline layout.

```ds
<Paragraph>Hello <Strong>World</Strong>!</Paragraph>
```

```ds expected
<Paragraph>
    Hello <Strong>World</Strong>!
</Paragraph>;
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
<Container>{/* XOXO: something something add content */}</Container>;
```

## Complex Expression Children

### map expression in children

Map expressions can generate multiple elements.

```ds line-width=50
<List>{items.map((item) => <Item key={item.id} />)}</List>
```

```ds expected
<List>
    {items.map((item) => (
        <Item key={item.id} />
    ))}
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
        );
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
    {loading ? (
        <Spinner size="large" />
    ) : (
        <Content data={data} />
    )}
</div>;
```

### ternary branch comments

Trailing JSX branch comments format at conditional branch boundaries.

```ds line-width=40
<div>{isVideo ? <Video /> /* keep-video */ : <Image /> /* keep-image */}</div>
```

```ds expected
<div>
    {
        isVideo ? (
            <Video />
        ) : (
            /* keep-video */ <Image />
        ) /* keep-image */
    }
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
createPortal(<Modal isOpen={true} />, document.body);
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

## JSX Formatting Options

### single attribute per line forces expansion

When `single_attribute_per_line` is true, multiple attributes each get their own line.

```ds single-attribute-per-line=true
<Button variant="primary" size="large" />
```

```ds expected
<Button
    variant="primary"
    size="large"
/>;
```

### bracket same line keeps self-closing slash on its own line

For self-closing tags, `bracket_same_line` does not pull `/>` up onto the last attribute line.

```ds bracket-same-line=true line-width=30
<Button variant="primary" size="large" disabled />
```

```ds expected
<Button
    variant="primary"
    size="large"
    disabled
/>;
```

### single attr per line with bracket same line combined

Both options can be combined while still keeping the self-closing `/>` on its own line.

```ds single-attribute-per-line=true bracket-same-line=true
<Button variant="primary" size="large" onClick={handleClick} />
```

```ds expected
<Button
    variant="primary"
    size="large"
    onClick={handleClick}
/>;
```
