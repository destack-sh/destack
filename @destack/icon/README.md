# @destack/icon

Draw [Phosphor](https://phosphoricons.com) icons inline as SVG.

## Icons

`Icon` draws an icon by name in the current text color at 1em, hidden from assistive technology unless it has a label.

```tsx
import { Icon } from "@destack/icon";

<Icon name="trash" />;
<Icon name="trash" weight="bold" size="1.5rem" label="Delete" />;
<Icon name="github-logo" weight="fill" xstyle={styles.toolIcon} />;
```

## Weights

Each icon draws in six weights, regular by default.

```tsx
<Icon name="heart" weight="thin" />;
<Icon name="heart" weight="duotone" />;
```

## Loading

Each icon is its own module with all six weights, which builds import for an icon with a literal name and refuse for a computed name without `icon`.

```tsx
import check from "@destack/icon/phosphor/check-circle";
import circle from "@destack/icon/phosphor/circle";

<Icon name="trash" />; // the build passes the trash module as `icon`
<Icon icon={done() ? check : circle} />; // a choice between known icons
```

## Lazy loading

`LazyIcon` from `@destack/icon/lazy` draws any icon chosen at run time, loading its module on first draw and empty at its final size until it arrives.

```tsx
import { LazyIcon } from "@destack/icon/lazy";

<LazyIcon name={file().icon} />; // only modules importing LazyIcon carry the index of every icon
```

## Builds

`iconExtension` passes each `Icon` imported by name with a literal name the bodies of a static import of `@destack/icon/phosphor/<name>` in builds of every package whose dependency closure includes `@destack/icon`, and fails the build at the line and column of an `Icon` without `icon` whose name it cannot read.

```tsx
<Icon name="trash" />; // <Icon name="trash" icon={trash} /> with trash from @destack/icon/phosphor/trash
<Icon icon={properties.icon} />; // left as written
<Icon name={properties.name} />; // fails at 4:58, as do <Icon {...properties} /> and <icons.Icon name="trash" />
```

## Data

`bun run generate` writes one module per icon with its SVG body in each weight, and the `IconName` type, from the pinned `@phosphor-icons/core`, whose MIT license sits in `src/phosphor/LICENSE`.

```sh
bun run generate
```
