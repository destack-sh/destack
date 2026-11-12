# @destack/icon

Draw Phosphor icons by name, each one loaded only where it is used.

## Overview

`@destack/icon` is Phosphor's icons as one component: icons by name, one module per icon, and a build that imports only the icons a package uses.

Views draw icons:

- `Icon`: one icon in the current text color.
- `LazyIcon`: an icon chosen at run time, loaded on first draw.
- `@destack/icon/phosphor/<name>`: one icon's bodies in every weight.

Builds import only what is used:

- `iconExtension`: passes each literal-named `Icon` its module.

Draw an icon from its module:

```tsx
import { Icon } from "@destack/icon";
import trash from "@destack/icon/phosphor/trash";

// draw the bold trash icon, labelled for assistive technology
export const DeleteIcon = () => <Icon icon={trash} weight="bold" size="1.5rem" label="Delete" />;
```

## Reference

### Icon

`Icon` takes its icon by `name`, written as a literal, or as an imported `icon`.
A computed name without `icon` fails the build, and `LazyIcon` takes one instead.

| Property | What it is                                                                   |
| -------- | ---------------------------------------------------------------------------- |
| `name`   | the Phosphor icon's name, as an `IconName`                                   |
| `icon`   | the bodies imported from `@destack/icon/phosphor/<name>`                     |
| `weight` | `"thin"`, `"light"`, `"regular"`, `"bold"`, `"fill"` or `"duotone"`          |
| `size`   | the width and height as a CSS length, `1em` by default                       |
| `label`  | the accessible name, which exposes the icon as an image instead of hiding it |
| `xstyle` | StyleX styles, such as margins                                               |

## Examples

### Draw an icon chosen at run time

`LazyIcon` takes a name known only at run time.

```tsx
import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/icon/lazy";

// load the icon's module on first draw
export const FileIcon = (properties: { readonly name: IconName }) => (
    <LazyIcon name={properties.name} />
);
```

## Comparison

### Phosphor React

Names, weights and sizes follow `@phosphor-icons/react`.

| Phosphor React         | `@destack/icon`      | Difference                                                  |
| ---------------------- | -------------------- | ----------------------------------------------------------- |
| one component per icon | `Icon` with a `name` | The build imports only the icons a package uses.            |
| `alt` or `aria-hidden` | `label`              | Icons are hidden from assistive technology unless labelled. |
