# String Literals

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

TypeScript single quoted input normalizes to formatter quote style.

```ts:main.ts
const x = 'a'
```

```ts expected
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
