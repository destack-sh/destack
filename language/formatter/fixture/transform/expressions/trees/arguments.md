# Tree Arguments

## Calls

### tree in function call

Tree literals can be passed as function arguments.

```tspp
render(<App />)
```

```tspp expected
render(<App />);
```

### tree with props in function call

Tree literals with attributes can appear in function arguments.
Boolean expression attributes remain explicit.

```tspp
createPortal(<Modal isOpen={true} />, document.body)
```

```tspp expected
createPortal(<Modal isOpen={true} />, document.body);
```

### complex tree in function call breaks

Complex tree arguments break onto a new line.

```tspp line-width=40
render(<Container><Header /><Content /></Container>)
```

```tspp expected
render(
    <Container>
        <Header />
        <Content />
    </Container>,
);
```
