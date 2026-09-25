# Await Expressions

Await fixtures cover awaited calls, member chains, comments, and statement contexts.

## Await Forms

### await call

Await expressions keep a space after `await`.

```tspp
const result = await fetch(url)
```

```tspp expected
const result = await fetch(url);
```

### await member chain

Await applies to the full chain without extra parentheses.

```tspp
const value = await client.getUser(id).profile
```

```tspp expected
const value = await client.getUser(id).profile;
```

## Await in Arguments

### await inside call arguments

Await arguments format like normal expressions.

```tspp
const value = combine(await left(), await right())
```

```tspp expected
const value = combine(await left(), await right());
```

## Await Maybe

### await? expression

Error-propagating await keeps the `?` tight to `await`.

```tspp
const result = await? fetch(url)
```

```tspp expected
const result = await? fetch(url);
```

## Await Must

### await! expression

The `!` marker prints directly after `await`.

```tspp
const result = await! fetch(url)
```

```tspp expected
const result = await! fetch(url);
```
