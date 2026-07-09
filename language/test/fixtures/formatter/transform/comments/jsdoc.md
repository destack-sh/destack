# JSDoc

JSDoc fixtures cover declaration documentation formatting.

## Declarations

### jsdoc paragraph breaks

Paragraph breaks in declaration docs are preserved.

```ds:main.ds line-width=80
/**
 * creates the reader
 *
 * returns a configured value
 */
function readReader() {}
```

```ds expected
/**
 * Creates the reader
 *
 * Returns a configured value
 */
function readReader() {}
```

### jsdoc declaration comments

Declaration comments can collapse to single-line JSDoc.

```ds:main.ds line-width=80
/**
 * reads the value
 */
function read() {}

/**
 * already Capitalized
 */
const value = 1
```

```ds expected
/** Reads the value */
function read() {}

/** Already Capitalized */
const value = 1;
```

## Attachment

### jsdoc class and interface members

Member-level documentation is formatted.

```ds:main.ds line-width=80
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

```ds expected
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

```ds:main.ds line-width=80
/**
 * creates a reader
 * @param {string} name reader name
 * @returns {Reader} configured reader
 */
export function createReader(name: string): Reader { return new Reader(name) }
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * reads config
 * @description resolves defaults and validates values
 * @returns {Config} active config
 */
function readConfig() {}
```

```ds expected
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

Recognized tag aliases normalize to canonical tag names.

```ds:main.ds line-width=80
/**
 * @arg {number} count item count
 * @return {number} total count
 * @exception {RangeError} when count is invalid
 */
function total(count: number): number { return count }
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * creates options
 * @param {object} options builder options
 * @property {string} options.name display name
 * @property {boolean} [options.enabled=true] whether the option is enabled
 */
function createOptions(options: Options) {}
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * do not format me
 */
read()
```

```ds expected
/**
 * do not format me
 */
read();
```

## Tags

### jsdoc parameter default description

JSDoc parameter defaults keep the same normalized description shape.

```ds:main.ds line-width=80
/**
 * @param {string} x The string to parse as a number
 * @param {boolean} [int=true] Whether to parse as an integer or float. Default
 *   is `true`.
 * @returns {number} The parsed number
 */
function parseNumber(x, int) {}
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * @param {number} second second value
 * @param {number} first first value
 * @returns {number} total
 */
function add(first, second) { return first + second }
```

```ds expected
/**
 * @param {number} second Second value
 * @param {number} first First value
 * @returns {number} Total
 */
function add(first, second) {
    return first + second;
}
```

### jsdoc dash descriptions

Dash-separated tag descriptions format as declaration docs.

```ds:main.ds line-width=80 indent-width=2
/**
 * @param {string} name - the user name
 */
function greet(name) {}

/**
 * @returns {string} the greeting
 */
function hello() {}

/**
 * @param {number} x - 123 starts with number
 */
function read(x) {}
```

```ds expected
/** @param {string} name - The user name */
function greet(name) {}

/** @returns {string} The greeting */
function hello() {}

/** @param {number} x - 123 starts with number */
function read(x) {}
```

### jsdoc single-line tags

Short recognized tags collapse to single-line JSDoc.

```ds:main.ds line-width=80 indent-width=2
/**
 * @deprecated
 */
function old() {}

/**
 * @returns {number}
 */
function count() {}
```

```ds expected
/** @deprecated */
function old() {}

/** @returns {number} */
function count() {}
```

### jsdoc tag blank line descriptions

Blank lines between tags and descriptions preserve continuation indentation.

```ds:main.ds line-width=80 indent-width=2
/**
 * @param {string} name
 *
 * Description after blank line for a param tag.
 */
function withParamBlank(name) {}
```

```ds expected
/**
 * @param {string} name
 *
 *   Description after blank line for a param tag.
 */
function withParamBlank(name) {}
```

### jsdoc typeless parameter docs

Typeless parameter and return tags format their descriptions.

```ds:main.ds line-width=80
/**
 * parses a source file
 * @param filename source file name
 * @param sourceText source text
 * @returns parsed program
 */
function parse(filename: string, sourceText: string) { return sourceText }
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * @param second second value
 * @param first first value
 */
function pair(first: number, second: number) {}
```

```ds expected
/**
 * @param second Second value
 * @param first First value
 */
function pair(first: number, second: number) {}
```

### jsdoc multiline param type

Multiline type text is preserved while comment leaders are stripped.

```ds:main.ds line-width=80
/**
 * @param {{
 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
 * }} props
 */
function boundary(props) {}
```

```ds expected
/**
 * @param {{
 *     failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
 * }} props
 */
function boundary(props) {}
```

### jsdoc type parameter docs

Type parameter documentation formats as prose.

```ds:main.ds line-width=80
/**
 * @typeParam T wrapped value
 * @param {T} value input value
 * @returns {T} same value
 */
function identity<T>(value: T) { return value }
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * @remarks this is longer documentation.
 * @deprecated use `next` instead.
 * @see {@link next}
 * @default {"enabled":true}
 */
function old() {}
```

```ds expected
/**
 * @remarks
 *   This is longer documentation.
 * @deprecated use `next` instead.
 * @see {@link next}
 * @default { "enabled": true }
 */
function old() {}
```

### jsdoc default tag preserves long values

Default tags keep their value on the tag line.

```ds:main.ds line-width=60
/**
 * @default <span v-pre>`"{{browser}}-mv{{manifestVersion}}{{modeSuffix}}"`</span>
 */
function outDirTemplate() {}
```

```ds expected
/** @default <span v-pre>`"{{browser}}-mv{{manifestVersion}}{{modeSuffix}}"`</span> */
function outDirTemplate() {}
```

### jsdoc example default tag

Example tags keep following default values unwrapped.

```ds:main.ds line-width=60 indent-width=2
/**
 * @example
 *   {{browser}} -mv{{manifestVersion}}
 *
 * @default <span v-pre>`"{{browser}}-mv{{manifestVersion}}{{modeSuffix}}"`</span>
 */
function outDirTemplate() {}
```

```ds expected
/**
 * @example
 *   {{browser}} -mv{{manifestVersion}}
 * @default <span v-pre>`"{{browser}}-mv{{manifestVersion}}{{modeSuffix}}"`</span>
 */
function outDirTemplate() {}
```

## Markdown

### jsdoc markdown description

Markdown descriptions normalize emphasis, lists, links, and wrapping.

```ds:main.ds line-width=70
/**
 * __bold__ and *italic* text with {@link VeryLongTargetName} that should wrap around the configured width.
 *
 * - first item
 *
 * - second item
 */
function describe() {}
```

```ds expected
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

```ds:main.ds line-width=80
/**
 * @example
 * ```ts
 * const result=call( 1,2 )
 * ```
 */
function example() {}
```

```ds expected
/**
 * @example
 *     ```ts
 *     const result = call(1, 2);
 *     ```
 */
function example() {}
```

### jsdoc example fenced Destack code

Destack code fences are formatted as embedded code.

```ds:main.ds line-width=80
/**
 * @example
 * ```ds
 * const result=match(state){Ready=>"go";Loading=>"wait";_=>"unknown"}
 * ```
 */
function example() {}
```

```ds expected
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

### jsdoc example typed snippets

Example snippets keep generic syntax intact.

```ds:main.ds line-width=80 indent-width=2
/**
 * @example
 * await storage.getItem<number>("key");
 */
function foo() {}

/**
 * @example
 * const result = override<{ opt: string }>({ opt: "value" });
 */
function bar() {}
```

```ds expected
/**
 * @example
 *   await storage.getItem<number>("key");
 */
function foo() {}

/**
 * @example
 *   const result = override<{ opt: string }>({ opt: "value" });
 */
function bar() {}
```

## Indentation

### jsdoc multiline param type with tabs

Tab-indented JSDoc keeps tab continuation indentation.

```ds:main.ds line-width=80 indent-style=tab
class Renderer {
	/**
	 * @param {{
	 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
	 * }} props
	 */
	boundary(props) {}
}
```

```ds expected
class Renderer {
	/**
	 * @param {{
	 * 	failed?: (renderer: Renderer, error: unknown, reset: () => void) => void;
	 * }} props
	 */
	boundary(props) {}
}
```
