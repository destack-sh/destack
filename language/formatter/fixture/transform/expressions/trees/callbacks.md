# Tree Callbacks

## Multiple Callbacks

### element with multiple callback props

Multiple callbacks break element to multi-line.

```tspp line-width=50
<Button onClick={handleClick} onHover={handleHover} onFocus={handleFocus} />
```

```tspp expected
<Button
    onClick={handleClick}
    onHover={handleHover}
    onFocus={handleFocus}
/>;
```

### callback with inline arrow function

Inline arrow functions as callbacks.

```tspp line-width=60
<Button onClick={() => setOpen(true)} onClose={() => setOpen(false)} />
```

```tspp expected
<Button
    onClick={() => setOpen(true)}
    onClose={() => setOpen(false)}
/>;
```
