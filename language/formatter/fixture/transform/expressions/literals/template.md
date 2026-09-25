# Template Literals

## Template Literals

### template literal

Template literals use backticks.

```tspp
const x = `hello`
```

```tspp expected
const x = `hello`;
```

### template with interpolation

Interpolations use `${...}` syntax.

```tspp
const x = `hello ${name}`
```

```tspp expected
const x = `hello ${name}`;
```

### template with expression

Expressions can be interpolated.

```tspp
const x = `result: ${a + b}`
```

```tspp expected
const x = `result: ${a + b}`;
```

### template with multiple interpolations

Multiple interpolations can appear in one template.

```tspp
const x = `${a} + ${b} = ${a + b}`
```

```tspp expected
const x = `${a} + ${b} = ${a + b}`;
```

### template with nested template

Templates can be nested within interpolations.

```tspp
const x = `outer ${`inner ${value}`} end`
```

```tspp expected
const x = `outer ${`inner ${value}`} end`;
```

### multiline template literal

Multiline templates preserve line breaks.

```tspp
const x = `line1
line2
line3`
```

```tspp expected
const x = `line1
line2
line3`;
```

### template with function call

Function calls can be interpolated.

```tspp
const x = `result: ${getValue()}`
```

```tspp expected
const x = `result: ${getValue()}`;
```

### template with ternary

Ternary expressions can be interpolated.

```tspp
const x = `status: ${active ? "on" : "off"}`
```

```tspp expected
const x = `status: ${active ? "on" : "off"}`;
```

### template literal with chained expression breaks

Long chained expressions inside template literals break cleanly.

```tspp:main.tspp line-width=80
const A = {
  "--theme-primary": `hsl(${theme?.activeColor[
    mode === "dark" ? "dark" : "light"
  ]})`,
};
```

```tspp expected
const A = {
    "--theme-primary": `hsl(${theme?.activeColor[
        mode === "dark" ? "dark" : "light"
    ]})`,
};
```

### multiline template interpolation comment

Interpolations hug their braces while inner comments stay indented from the template segment.

```tspp:main.tspp
const css = `
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`;
```

```tspp expected
const css = `
  color: ${theme?.activeColor[
      // selected mode
      mode === "dark" ? "dark" : "light"
  ]};
`;
```

### multiline template interpolation comments

Each interpolation derives indentation from the preceding template segment.

```tspp:main.tspp
const css = `
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
    background: ${theme?.backgroundColor[
      // selected mode
      mode === "dark" ? "dark" : "light"
    ]};
`;
```

```tspp expected
const css = `
  color: ${theme?.activeColor[
      // selected mode
      mode === "dark" ? "dark" : "light"
  ]};
    background: ${theme?.backgroundColor[
        // selected mode
        mode === "dark" ? "dark" : "light"
    ]};
`;
```

## Tagged Templates

### tagged template

Tagged templates apply a function to the template.

```tspp
sql`SELECT * FROM users`
```

```tspp expected
sql`SELECT * FROM users`;
```

### tagged template with interpolation

Tagged templates can include interpolations.

```tspp
sql`SELECT * FROM ${table} WHERE id = ${id}`
```

```tspp expected
sql`SELECT * FROM ${table} WHERE id = ${id}`;
```

### html tagged template

HTML tagged templates are common for rendering.

```tspp
html`<div class="${className}">${content}</div>`
```

```tspp expected
html`<div class="${className}">${content}</div>`;
```

### css tagged template

CSS tagged templates are common for styling.

```tspp
css`color: ${color}; font-size: ${size}px;`
```

```tspp expected
css`color: ${color}; font-size: ${size}px;`;
```
