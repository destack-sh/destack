# Await Expressions

Tests for await expression formatting.

## Basic Await

### await call

Await expressions keep a space after `await`.

```ds
const result = await fetch(url)
```

```ds expected
const result = await fetch(url);
```

### await member chain

Await applies to the full chain without extra parentheses.

```ds
const value = await client.getUser(id).profile
```

```ds expected
const value = await client.getUser(id).profile;
```

## Await in Arguments

### await inside call arguments

Await arguments format like normal expressions.

```ds
const value = combine(await left(), await right())
```

```ds expected
const value = combine(await left(), await right());
```

## Await Maybe

### await? expression

Error-propagating await keeps the `?` tight to `await`.

```ds
const result = await? fetch(url)
```

```ds expected
const result = await? fetch(url);
```
