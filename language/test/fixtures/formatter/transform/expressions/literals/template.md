# Template Literals

## Template Literals

### template literal

Template literals use backticks.

```ds
const x = `hello`
```

```ds expected
const x = `hello`;
```

### template with interpolation

Interpolations use `${...}` syntax.

```ds
const x = `hello ${name}`
```

```ds expected
const x = `hello ${name}`;
```

### template with expression

Expressions can be interpolated.

```ds
const x = `result: ${a + b}`
```

```ds expected
const x = `result: ${a + b}`;
```

### template with multiple interpolations

Multiple interpolations can appear in one template.

```ds
const x = `${a} + ${b} = ${a + b}`
```

```ds expected
const x = `${a} + ${b} = ${a + b}`;
```

### template with nested template

Templates can be nested within interpolations.

```ds
const x = `outer ${`inner ${value}`} end`
```

```ds expected
const x = `outer ${`inner ${value}`} end`;
```

### multiline template literal

Multiline templates preserve line breaks.

```ds
const x = `line1
line2
line3`
```

```ds expected
const x = `line1
line2
line3`;
```

### template with function call

Function calls can be interpolated.

```ds
const x = `result: ${getValue()}`
```

```ds expected
const x = `result: ${getValue()}`;
```

### template with ternary

Ternary expressions can be interpolated.

```ds
const x = `status: ${active ? "on" : "off"}`
```

```ds expected
const x = `status: ${active ? "on" : "off"}`;
```

### template literal with chained expression breaks

Long chained expressions inside template literals break cleanly.

```ts:main.ts line-width=80
const A = {
  "--theme-primary": `hsl(${theme?.activeColor[
    mode === "dark" ? "dark" : "light"
  ]})`,
};
```

```ts expected
const A = {
    "--theme-primary": `hsl(${
        theme?.activeColor[mode === "dark" ? "dark" : "light"]
    })`,
};
```

### multiline template interpolation comment

Comments inside an indented multiline template interpolation keep the expression indented under `${`.

```ts:main.ts
const css = `
  color: ${theme?.activeColor[
    // selected mode
    mode === "dark" ? "dark" : "light"
  ]};
`;
```

```ts expected
const css = `
  color: ${
      theme?.activeColor[
          // selected mode
          mode === "dark" ? "dark" : "light"
      ]
  };
`;
```

### multiline template interpolation comments

Each interpolation derives indentation from the preceding template segment.

```ts:main.ts
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

```ts expected
const css = `
  color: ${
      theme?.activeColor[
          // selected mode
          mode === "dark" ? "dark" : "light"
      ]
  };
    background: ${
        theme?.backgroundColor[
            // selected mode
            mode === "dark" ? "dark" : "light"
        ]
    };
`;
```

## Tagged Templates

### tagged template

Tagged templates apply a function to the template.

```ds
sql`SELECT * FROM users`
```

```ds expected
sql`SELECT * FROM users`;
```

### tagged template with interpolation

Tagged templates can include interpolations.

```ds
sql`SELECT * FROM ${table} WHERE id = ${id}`
```

```ds expected
sql`SELECT * FROM ${table} WHERE id = ${id}`;
```

### html tagged template

HTML tagged templates are common for rendering.

```ds
html`<div class="${className}">${content}</div>`
```

```ds expected
html`<div class="${className}">${content}</div>`;
```

### css tagged template

CSS tagged templates are common for styling.

```ds
css`color: ${color}; font-size: ${size}px;`
```

```ds expected
css`color: ${color}; font-size: ${size}px;`;
```

## Tagged Templates

### tagged template literal

Tagged templates apply a function to a template literal.

```ds
sql`SELECT * FROM users`
```

```ds expected
sql`SELECT * FROM users`;
```

### tagged template with interpolation

Tagged templates can include interpolated expressions.

```ds
html`<div>${content}</div>`
```

```ds expected
html`<div>${content}</div>`;
```

