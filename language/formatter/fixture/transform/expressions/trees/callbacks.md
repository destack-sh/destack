# Tree Callbacks

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
