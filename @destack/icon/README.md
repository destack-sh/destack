# @destack/icon

Draw Phosphor icons by name, each one loaded only where it is used.

## Overview

`Icon` draws a Phosphor icon inline in the current text color, and the build imports only the icons a package uses.

- **Icon**: one icon by `name`, with Phosphor's `weight` and a `size`; a `label` exposes it as an image, otherwise assistive technology skips it.
- **Icon module**: one icon's bodies in every weight, at `@destack/icon/phosphor/<name>`.
- **LazyIcon**: an icon chosen at run time, its module loaded on first draw.
- **iconExtension**: the build extension passing each literal-named `Icon` its module, failing the build for a computed name without `icon`.

```tsx
import { Icon } from "@destack/icon";
import trash from "@destack/icon/phosphor/trash";

const DeleteIcon = () => <Icon icon={trash} weight="bold" size="1.5rem" label="Delete" />;
```

## Examples

### Draw an icon by name

A literal `name` is enough, as the build passes the named icon's module.

```tsx
import { Icon } from "@destack/icon";

const Trash = () => <Icon name="trash" weight="bold" />;
```

### Draw an icon chosen at run time

`LazyIcon` loads the named icon's module the first time it draws.

```tsx
import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/icon/lazy";

const FileIcon = (properties: { readonly name: IconName }) => <LazyIcon name={properties.name} />;
```

## Comparison

### Phosphor React

Names, weights and sizes follow `@phosphor-icons/react`; two things differ.

- One `Icon` component takes the icon by name, and the build imports only the icons a package uses.
- An icon without a `label` is hidden from assistive technology, and one with a label is an image.
