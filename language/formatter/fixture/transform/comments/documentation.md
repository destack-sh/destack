# Documentation

Documentation is rendered from the parsed documentation attached to each DIR node.

## Canonical form

### Render block documentation as line documentation

Documentation uses one line marker and separates prose from structured entries.

```tspp:main.tspp line-width=80
/**
 * Map one value.
 * @typeParam T: The value type.
 * @param value The value to map.
 * @example
 * ```tspp
 * map(value)
 * ```
 */
function map<T>(value: T): T { return value }
```

```tspp expected
/// Map one value.
///
/// @typeParam T - The value type.
/// @param value - The value to map.
///
/// @example
/// ```tspp
/// map(value);
/// ```
function map<T>(value: T): T {
    return value;
}
```

### Normalize parameter separators

Space, dash, and colon separators have one canonical output.

```tspp:main.tspp line-width=80
/// Send one request.
/// @param first The first request.
/// @param second - The second request.
/// @param third: The third request.
function send(first: Request, second: Request, third: Request) {}
```

```tspp expected
/// Send one request.
///
/// @param first - The first request.
/// @param second - The second request.
/// @param third - The third request.
function send(first: Request, second: Request, third: Request) {}
```

### Begin empty documentation with structured entries

Structured entries do not acquire a leading empty documentation line.

```tspp:main.tspp line-width=80
/// @param request The request to send.
function send(request: Request) {}
```

```tspp expected
/// @param request - The request to send.
function send(request: Request) {}
```

## Markdown

### Preserve documentation sections

Markdown headings and paragraphs remain ordinary documentation content.

```tspp:main.tspp line-width=80
/// Read one value.
///
/// # Errors
///
/// Returns `InvalidInput` when the source is empty.
function read(): Result<Value, InvalidInput> {}
```

```tspp expected
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

```tspp:main.tspp line-width=80
/** Read one binding. */
@binding("tspp.read")
declare function readBinding(): string
```

```tspp expected
/// Read one binding.
@binding("tspp.read")
declare function readBinding(): string;
```
