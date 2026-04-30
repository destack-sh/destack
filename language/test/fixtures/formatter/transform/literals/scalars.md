# Scalar Literals

Literal fixtures cover strings, templates, numeric literals, booleans, nullish literals, and regular expressions.

## Strings

### double quoted string

Double quoted strings are preserved as-is.

```ds
const x = "hello"
```

```ds expected
const x = "hello";
```

### single quoted string

Single quoted input normalizes to formatter quote style.

```ds
const x = 'a'
```

```ds expected
const x = "a";
```

### empty string

Empty strings are preserved.

```ds
const x = ""
```

```ds expected
const x = "";
```

### string with spaces

String content is preserved exactly.

```ds
const x = "hello world"
```

```ds expected
const x = "hello world";
```

## Escape Sequences

### string with escape sequences

Escape sequences are preserved.

```ds
const x = "hello\nworld"
```

```ds expected
const x = "hello\nworld";
```

### string with tab

Tab escapes are preserved.

```ds
const x = "hello\tworld"
```

```ds expected
const x = "hello\tworld";
```

### string with backslash

Backslash escapes are preserved.

```ds
const x = "path\\to\\file"
```

```ds expected
const x = "path\\to\\file";
```

### string with quotes

Escaped quotes are preserved.

```ds
const x = "say \"hello\""
```

```ds expected
const x = 'say "hello"';
```

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

## String Concatenation

### string concatenation

String concatenation uses `+` operator.

```ds
"hello" + " " + "world"
```

```ds expected
"hello" + " " + "world";
```

### string concat with variables

Variables can be concatenated with strings.

```ds
prefix + name + suffix
```

```ds expected
prefix + name + suffix;
```

## String as Arguments

### string as function argument

Strings can be passed directly as arguments.

```ds
foo("hello")
```

```ds expected
foo("hello");
```

### string in array

Strings can be array elements.

```ds
["a", "b", "c"]
```

```ds expected
["a", "b", "c"];
```

### string in object

Strings can be object property values.

```ds
const x = { name: "test" }
```

```ds expected
const x = { name: "test" };
```

## String Methods

### string method call

Methods can be called on string literals.

```ds
"hello".toUpperCase()
```

```ds expected
"hello".toUpperCase();
```

### string method chain

Method chains on strings work normally.

```ds
"  hello  ".trim().toUpperCase()
```

```ds expected
"  hello  ".trim().toUpperCase();
```

### template method call

Properties can be accessed on template literals.

```ds
`hello ${name}`.length
```

```ds expected
`hello ${name}`.length;
```

## Numeric Literals

### integer

Integer literals are preserved.

```ds
const x = 42
```

```ds expected
const x = 42;
```

### negative integer

Negative numbers use unary minus.

```ds
const x = -42
```

```ds expected
const x = -42;
```

### float

Floating point literals are preserved.

```ds
const x = 3.14
```

```ds expected
const x = 3.14;
```

### scientific notation

Scientific notation is preserved.

```ds
const x = 1e10
```

```ds expected
const x = 1e10;
```

### hexadecimal

Hex literals use `0x` prefix.

```ds
const x = 0xFF
```

```ds expected
const x = 0xff;
```

### octal

Octal literals use `0o` prefix.

```ds
const x = 0o17
```

```ds expected
const x = 0o17;
```

### binary

Binary literals use `0b` prefix.

```ds
const x = 0b1010
```

```ds expected
const x = 0b1010;
```

### bigint

BigInt literals use `n` suffix.

```ds
const x = 42n
```

```ds expected
const x = 42n;
```

### numeric separator

Numeric separators improve readability.

```ds
const x = 1_000_000
```

```ds expected
const x = 1_000_000;
```

## Boolean Literals

### true literal

Boolean true is preserved.

```ds
const x = true
```

```ds expected
const x = true;
```

### false literal

Boolean false is preserved.

```ds
const x = false
```

```ds expected
const x = false;
```

## Null and Undefined

### null literal

Null is preserved as-is.

```ds
const x = null
```

```ds expected
const x = null;
```

### undefined literal

Undefined is preserved as-is.

```ds
const x = undefined
```

```ds expected
const x = undefined;
```

## Regex Literals

### regex literal

Regex literals use forward slashes.

```ds
const x = /pattern/
```

```ds expected
const x = /pattern/;
```

### regex with flags

Flags follow the closing slash.

```ds
const x = /\d+/g
```

```ds expected
const x = /\d+/g;
```

### regex with multiple flags

Multiple flags can be combined.

```ds
const x = /hello/gi
```

```ds expected
const x = /hello/gi;
```

### regex with escaped pattern

Complex patterns are preserved exactly.

```ds
const x = /^[a-z]+$/i
```

```ds expected
const x = /^[a-z]+$/i;
```

## Long Strings

### long string assignment breaks after operator

Long string declarator values break after `=` when they exceed line width.

```ds line-width=40
const msg = "This is a very long string that exceeds the line width but should not be broken"
```

```ds expected
const msg =
    "This is a very long string that exceeds the line width but should not be broken";
```

### long template literal assignment stays inline

Long template literal declarator values stay inline even when they exceed line width.

```ds line-width=40
const msg = `This is a very long template literal that exceeds the line width but should not be broken`
```

```ds expected
const msg = `This is a very long template literal that exceeds the line width but should not be broken`;
```
