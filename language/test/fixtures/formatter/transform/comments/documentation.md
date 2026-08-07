# Documentation

Documentation is rendered from the parsed documentation attached to each DIR node.

## Canonical form

### Render block documentation as line documentation

Documentation uses one line marker and separates prose from structured entries.

```ds:main.ds line-width=80
/**
 * Map one value.
 * @typeParam T: The value type.
 * @param value The value to map.
 * @example
 * ```ds
 * map(value)
 * ```
 */
function map<T>(value: T): T { return value }
```

```ds expected
/// Map one value.
///
/// @typeParam T - The value type.
/// @param value - The value to map.
///
/// @example
/// ```ds
/// map(value);
/// ```
function map<T>(value: T): T {
    return value;
}
```

### Normalize parameter separators

Space, dash, and colon separators have one canonical output.

```ds:main.ds line-width=80
/// Send one request.
/// @param first The first request.
/// @param second - The second request.
/// @param third: The third request.
function send(first: Request, second: Request, third: Request) {}
```

```ds expected
/// Send one request.
///
/// @param first - The first request.
/// @param second - The second request.
/// @param third - The third request.
function send(first: Request, second: Request, third: Request) {}
```

### Begin empty documentation with structured entries

Structured entries do not acquire a leading empty documentation line.

```ds:main.ds line-width=80
/// @param request The request to send.
function send(request: Request) {}
```

```ds expected
/// @param request - The request to send.
function send(request: Request) {}
```

## Markdown

### Preserve documentation sections

Markdown headings and paragraphs remain ordinary documentation content.

```ds:main.ds line-width=80
/// Read one value.
///
/// # Errors
///
/// Returns `InvalidInput` when the source is empty.
function read(): Result<Value, InvalidInput> {}
```

```ds expected
/// Read one value.
///
/// # Errors
///
/// Returns `InvalidInput` when the source is empty.
function read(): Result<Value, InvalidInput> {}
```

## Attachment

### Format decorated declaration documentation

Documentation remains attached through decorators.

```ds:main.ds line-width=80
/** Read one binding. */
@binding("destack.read")
declare function readBinding(): string
```

```ds expected
/// Read one binding.
@binding("destack.read")
declare function readBinding(): string;
```
