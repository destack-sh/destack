# JSDoc

Tests for supported declaration documentation formatting.

## Declarations

### jsdoc paragraph breaks

Paragraph breaks in declaration docs are preserved.

```ts:main.ts jsdoc=true line-width=80
/**
 * creates the reader
 *
 * returns a configured value
 */
function readReader() {}
```

```ts expected
/**
 * Creates the reader
 *
 * Returns a configured value
 */
function readReader() {}
```

### jsdoc simple declarations

Declaration comments can collapse to single-line JSDoc.

```ts:main.ts jsdoc=true line-width=80
/**
 * reads the value
 */
function read() {}

/**
 * already Capitalized
 */
const value = 1
```

```ts expected
/** Reads the value */
function read() {}

/** Already Capitalized */
const value = 1;
```

## Attachment

### jsdoc class and interface members

Member-level documentation is formatted.

```ts:main.ts jsdoc=true line-width=80
class Box {
    /**
     * count value
     */
    value = 1

    /**
     * gets value
     */
    get() { return this.value }
}

interface Shape {
    /**
     * display name
     */
    name: string
}
```

```ts expected
class Box {
    /** Count value */
    value = 1;

    /** Gets value */
    get() {
        return this.value;
    }
}

interface Shape {
    /** Display name */
    name: string;
}
```

### jsdoc exported declarations

JSDoc on exported declarations formats with the declaration.

```ts:main.ts jsdoc=true line-width=80
/**
 * creates a reader
 * @param {string} name reader name
 * @returns {Reader} configured reader
 */
export function createReader(name: string): Reader { return new Reader(name) }
```

```ts expected
/**
 * Creates a reader
 *
 * @param {string} name Reader name
 * @returns {Reader} Configured reader
 */
export function createReader(name: string): Reader {
    return new Reader(name);
}
```

### jsdoc description tag merge

Description tags merge into the leading description.

```ts:main.ts jsdoc=true line-width=80
/**
 * reads config
 * @description resolves defaults and validates values
 * @returns {Config} active config
 */
function readConfig() {}
```

```ts expected
/**
 * Reads config
 *
 * Resolves defaults and validates values
 *
 * @returns {Config} Active config
 */
function readConfig() {}
```

### jsdoc tag aliases

Supported tag aliases normalize to canonical tag names.

```ts:main.ts jsdoc=true line-width=80
/**
 * @arg {number} count item count
 * @return {number} total count
 * @exception {RangeError} when count is invalid
 */
function total(count: number): number { return count }
```

```ts expected
/**
 * @param {number} count Item count
 * @returns {number} Total count
 * @throws {RangeError} When count is invalid
 */
function total(count: number): number {
    return count;
}
```

### jsdoc property tags

Property tags format nested option documentation.

```ts:main.ts jsdoc=true line-width=80
/**
 * creates options
 * @param {object} options builder options
 * @property {string} options.name display name
 * @property {boolean} [options.enabled=true] whether the option is enabled
 */
function createOptions(options: Options) {}
```

```ts expected
/**
 * Creates options
 *
 * @param {object} options Builder options
 * @property {string} options.name Display name
 * @property {boolean} [options.enabled=true] Whether the option is enabled.
 *   Default is `true`
 */
function createOptions(options: Options) {}
```

### non-declaration jsdoc

JSDoc comments that do not document declarations are preserved.

```ts:main.ts jsdoc=true line-width=80
/**
 * do not format me
 */
read()
```

```ts expected
/**
 * do not format me
 */
read();
```

## Tags

### jsdoc parameter default description

JSDoc parameter defaults keep the same normalized description shape.

```ts:main.ts jsdoc=true line-width=80
/**
 * @param {string} x The string to parse as a number
 * @param {boolean} [int=true] Whether to parse as an integer or float. Default
 *   is `true`.
 * @returns {number} The parsed number
 */
function parseNumber(x, int) {}
```

```ts expected
/**
 * @param {string} x The string to parse as a number
 * @param {boolean} [int=true] Whether to parse as an integer or float. Default
 *   is `true`
 * @returns {number} The parsed number
 */
function parseNumber(x, int) {}
```

### jsdoc typed parameter order

Typed param tags keep source order.

```ts:main.ts jsdoc=true line-width=80
/**
 * @param {number} second second value
 * @param {number} first first value
 * @returns {number} total
 */
function add(first, second) { return first + second }
```

```ts expected
/**
 * @param {number} second Second value
 * @param {number} first First value
 * @returns {number} Total
 */
function add(first, second) {
    return first + second;
}
```

### jsdoc typeless parameter docs

Typeless parameter and return tags format their descriptions.

```ts:main.ts jsdoc=true line-width=80
/**
 * parses a source file
 * @param filename source file name
 * @param sourceText source text
 * @returns parsed program
 */
function parse(filename: string, sourceText: string) { return sourceText }
```

```ts expected
/**
 * Parses a source file
 *
 * @param filename Source file name
 * @param sourceText Source text
 * @returns Parsed program
 */
function parse(filename: string, sourceText: string) {
    return sourceText;
}
```

### jsdoc typeless parameter order

Typeless param tags keep source order.

```ts:main.ts jsdoc=true line-width=80
/**
 * @param second second value
 * @param first first value
 */
function pair(first: number, second: number) {}
```

```ts expected
/**
 * @param second Second value
 * @param first First value
 */
function pair(first: number, second: number) {}
```

### jsdoc multiline param type

Multiline type text is preserved while comment leaders are stripped.

```ts:main.ts jsdoc=true line-width=80
/**
 * @param {{
 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
 * }} props
 */
function boundary(props) {}
```

```ts expected
/**
 * @param {{
 *     failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
 * }} props
 */
function boundary(props) {}
```

### jsdoc type parameter docs

Type parameter documentation formats as prose.

```ts:main.ts jsdoc=true line-width=80
/**
 * @typeParam T wrapped value
 * @param {T} value input value
 * @returns {T} same value
 */
function identity<T>(value: T) { return value }
```

```ts expected
/**
 * @typeParam T Wrapped value
 * @param {T} value Input value
 * @returns {T} Same value
 */
function identity<T>(value: T) {
    return value;
}
```

### jsdoc remarks and metadata

Metadata and remarks keep their expected line shapes.

```ts:main.ts jsdoc=true line-width=80
/**
 * @remarks this is longer documentation.
 * @deprecated use `next` instead.
 * @see {@link next}
 * @default {"enabled":true}
 */
function old() {}
```

```ts expected
/**
 * @remarks
 *   This is longer documentation.
 * @deprecated use `next` instead.
 * @see {@link next}
 * @default { "enabled": true }
 */
function old() {}
```

## Markdown

### jsdoc markdown description

Markdown descriptions normalize emphasis, lists, links, and wrapping.

```ts:main.ts jsdoc=true line-width=70
/**
 * __bold__ and *italic* text with {@link VeryLongTargetName} that should wrap around the configured width.
 *
 * - first item
 *
 * - second item
 */
function describe() {}
```

```ts expected
/**
 * **bold** and _italic_ text with {@link VeryLongTargetName} that
 * should wrap around the configured width.
 *
 * - First item
 * - Second item
 */
function describe() {}
```

### jsdoc example fenced code

Example code blocks are formatted as embedded code.

```ts:main.ts jsdoc=true line-width=80
/**
 * @example
 * ```ts
 * const result=call( 1,2 )
 * ```
 */
function example() {}
```

```ts expected
/**
 * @example
 *     ```ts
 *     const result = call(1, 2);
 *     ```
 */
function example() {}
```

### jsdoc example fenced destack code

Destack code fences are formatted as embedded code.

```ts:main.ts jsdoc=true line-width=80
/**
 * @example
 * ```ds
 * const result=match(state){Ready=>"go";Loading=>"wait";_=>"unknown"}
 * ```
 */
function example() {}
```

```ts expected
/**
 * @example
 *     ```ds
 *     const result = match (state) {
 *         Ready => "go"
 *         Loading => "wait"
 *         _ => "unknown"
 *     };
 *     ```
 */
function example() {}
```

## Indentation

### jsdoc multiline param type with tabs

Tab-indented JSDoc keeps tab continuation indentation.

```ts:main.ts jsdoc=true line-width=80 indent-style=tab
class Renderer {
	/**
	 * @param {{
	 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
	 * }} props
	 */
	boundary(props) {}
}
```

```ts expected
class Renderer {
	/**
	 * @param {{
	 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
	 * }} props
	 */
	boundary(props) {}
}
```
