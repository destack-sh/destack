# Tree Arguments

## Calls

### tree in function call

Tree literals can be passed as function arguments.

```ds
render(<App />)
```

```ds expected
render(<App />);
```

### tree with props in function call

Tree literals with attributes can appear in function arguments.
Boolean expression attributes remain explicit.

```ds
createPortal(<Modal isOpen={true} />, document.body)
```

```ds expected
createPortal(<Modal isOpen={true} />, document.body);
```

### complex tree in function call breaks

Complex tree arguments break onto a new line.

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
