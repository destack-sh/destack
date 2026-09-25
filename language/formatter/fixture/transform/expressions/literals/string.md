# String Literals

## Strings

### double quoted string

Double quoted strings are preserved as-is.

```tspp
const x = "hello"
```

```tspp expected
const x = "hello";
```

### character literal

Character literals keep single quotes.

```tspp:main.tspp
const x = 'a'
```

```tspp expected
const x = 'a';
```

### empty string

Empty strings are preserved.

```tspp
const x = ""
```

```tspp expected
const x = "";
```

### string with spaces

String content is preserved exactly.

```tspp
const x = "hello world"
```

```tspp expected
const x = "hello world";
```


## Escape Sequences

### string with escape sequences

Escape sequences are preserved.

```tspp
const x = "hello\nworld"
```

```tspp expected
const x = "hello\nworld";
```

### string with tab

Tab escapes are preserved.

```tspp
const x = "hello\tworld"
```

```tspp expected
const x = "hello\tworld";
```

### string with backslash

Backslash escapes are preserved.

```tspp
const x = "path\\to\\file"
```

```tspp expected
const x = "path\\to\\file";
```

### string with quotes

Escaped quotes are preserved.

```tspp
const x = "say \"hello\""
```

```tspp expected
const x = "say \"hello\"";
```


## String Concatenation

### string concatenation

String concatenation uses `+` operator.

```tspp
"hello" + " " + "world"
```

```tspp expected
"hello" + " " + "world";
```

### string concat with variables

Variables can be concatenated with strings.

```tspp
prefix + name + suffix
```

```tspp expected
prefix + name + suffix;
```

## String as Arguments

### string as function argument

Strings can be passed directly as arguments.

```tspp
foo("hello")
```

```tspp expected
foo("hello");
```

### string in array

Strings can be array elements.

```tspp
["a", "b", "c"]
```

```tspp expected
["a", "b", "c"];
```

### string in object

Strings can be object property values.

```tspp
const x = { name: "test" }
```

```tspp expected
const x = { name: "test" };
```

## String Methods

### string method call

Methods can be called on string literals.

```tspp
"hello".toUpperCase()
```

```tspp expected
"hello".toUpperCase();
```

### string method chain

Method chains on strings work normally.

```tspp
"  hello  ".trim().toUpperCase()
```

```tspp expected
"  hello  ".trim().toUpperCase();
```

### template method call

Properties can be accessed on template literals.

```tspp
`hello ${name}`.length
```

```tspp expected
`hello ${name}`.length;
```


## Long Strings

### long string assignment breaks after operator

Long string declarator values break after `=` when they exceed line width.

```tspp line-width=40
const msg = "This is a very long string that exceeds the line width but should not be broken"
```

```tspp expected
const msg =
    "This is a very long string that exceeds the line width but should not be broken";
```

### long template literal assignment stays inline

Long template literal declarator values stay inline even when they exceed line width.

```tspp line-width=40
const msg = `This is a very long template literal that exceeds the line width but should not be broken`
```

```tspp expected
const msg = `This is a very long template literal that exceeds the line width but should not be broken`;
```
