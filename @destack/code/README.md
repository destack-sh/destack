# @destack/code

Parse, compare, patch and merge source code in the browser, in Bun and in workers.

## Overview

`@destack/code` is what Destack knows about source code as data, parsed by Lezer and CodeMirror's stream parsers.
`@destack/theme` colors the kinds of tokens, and `@destack/ui` draws code and what changed in it.

- `Source`: a text and where its lines start, edited without reading it again.
- `Language` and `Grammar`: a language, how to find it by a file or a tag, and what parses it.
- `Syntax` and `Tokens`: parsed code, whose edits parse only what they change, and the tokens of a range of it.
- `Difference`: what changed between two sources, by line and by word.
- `Patch`: the changes of files as git writes them, applied, inverted and left out hunk by hunk.
- `Merge`: two texts changed from a third, and their conflicts.

Parse a file and read the tokens of the lines on screen:

```ts
import { Language, Source } from "@destack/code";

// load the grammar of the file's language
const grammar = await Language.ofFile("plan.ts")?.load();
const source = Source.of("const port = 1;\n");

// parse it, read a line, and parse again only what an edit changes
const syntax = grammar?.parse(source);
const tokens = syntax?.tokens(source.range(0)); // keyword "const", plain " ", variable "port", …
const edited = syntax?.edit({ from: 13, to: 14, insert: "3000" });
```

## Reference

| Export               | What it is                                                                             |
| -------------------- | -------------------------------------------------------------------------------------- |
| `Source`             | A text with its lines: `of`, `line`, `range`, `slice`, `lineAt` and `edit`.            |
| `Range`, `Edit`      | Two places, and a replacement of the text between them.                                |
| `Language`           | A language: `all`, `of`, `ofFile`, `ofTag` and `load`.                                 |
| `Grammar`            | What parses a language, from `@destack/code/grammar/<name>` or `Language.load`.        |
| `Syntax`             | Parsed code: `tokens`, `parse` further, and `edit`.                                    |
| `Tokens`, `Token`    | The tokens of a range in typed arrays, read by line or within a range.                 |
| `Difference`         | What changed: `changes`, `words`, `hunks` and `edits`.                                 |
| `Hunk`, `HunkLine`   | Changed lines with some unchanged ones around them.                                    |
| `Patch`, `FilePatch` | Changes of files: `read`, `between`, `apply`, `invert`, `without` and `toString`.      |
| `Merge`, `Chunk`     | Two changed sources put together: `of`, `read`, `conflicts`, `resolve` and `toString`. |

## Examples

### Compare two texts

```ts
import { Difference, Source } from "@destack/code";

// find the changed lines, and the words that changed in the first of them
const difference = Difference.of(Source.of("port = 1\n"), Source.of("port = 3000\n"));
const [change] = difference.changes; // lines 0 to 1 in place of lines 0 to 1
const words = change === undefined ? [] : difference.words(change); // "1" became "3000"
```

### Take an agent's patch without one of its hunks

```ts
import { Patch, Source } from "@destack/code";

/** Apply a patch to a file, leaving its first hunk out. */
export function applyRest(written: string, text: string): string {
    const [file] = Patch.read(written).files;
    const [hunk] = file?.hunks ?? [];
    const rest = hunk === undefined ? file : file?.without(hunk);

    return rest?.apply(Source.of(text)).source.text ?? text;
}
```

### Merge an upgrade into a fork

```ts
import { Merge, Source } from "@destack/code";

/** Put a fork and an upgrade of one file together, the fork winning where both changed it. */
export function upgrade(released: string, forked: string, upgraded: string): string {
    let merge = Merge.of(Source.of(released), Source.of(forked), Source.of(upgraded));
    for (const conflict of merge.conflicts) {
        merge = merge.resolve(conflict, "ours");
    }

    return merge.toString();
}
```
