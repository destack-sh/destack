# @destack/icon

Name Phosphor icons and store their SVG bodies, each one loaded only where it is used.

## Overview

`@destack/icon` is Phosphor's icons as data, as `@phosphor-icons/core` is: their names, their catalog and one module of SVG bodies per icon.
Declarations validate icon names against it, and `@destack/ui` draws the icons.

- `IconName`: the name of a Phosphor icon, such as `book-open`, as a schema and a type.
- `IconBodies`: one icon's SVG body in each weight, and `IconBodies.load` to load one by name.
- `@destack/icon/phosphor/<name>`: one icon's bodies, imported where it is drawn.
- `@destack/icon/icons`: every icon with its categories and the words that find it, for pickers.

Validate a declared icon and load its bodies:

```ts
import { IconBodies, IconName } from "@destack/icon";

// validate the icon a type declares and load its bodies on first use
const name = IconName.parse("book-open");
const bodies = await IconBodies.load(name);
```

## Reference

| Export | What it is |
|---|---|
| `IconName` | The name of a Phosphor icon, as a schema and a type. |
| `IconWeight` | `"thin"`, `"light"`, `"regular"`, `"bold"`, `"fill"` or `"duotone"`. |
| `IconBodies` | One icon's SVG body in each weight, and `IconBodies.load(name)`. |
| `ICONS` | Every icon in name order, with its categories and search words, from `@destack/icon/icons`. |

## Examples

### Import one icon's bodies

Each icon's module holds its body in every weight, and bundlers keep only the icons imported.

```ts
import type { IconBodies } from "@destack/icon";
import trash from "@destack/icon/phosphor/trash";

// read the trash icon's bold body
const bodies: IconBodies = trash;
export const bold = bodies.bold;
```
