# Tree Arguments

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

JSX with attributes can appear in function arguments.
Boolean `{true}` uses shorthand.

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
