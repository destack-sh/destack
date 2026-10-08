# @destack/icon

`Icon` draws [Phosphor](https://phosphoricons.com) icons with `@phosphor-icons/react`'s `weight` and `size`, picked by `name`.

```tsx
<Icon name="trash" weight="bold" size="1.5rem" />; // Phosphor's <Trash weight="bold" size="1.5rem" />
<Icon name="trash" label="Delete" />; // hidden from assistive technology without a label
<Icon icon={done() ? check : circle} />; // an icon module from @destack/icon/phosphor/<name>
```

## Builds

`iconExtension` imports the module of each `Icon` with a literal name, and fails the build for a computed name without `icon`.

```tsx
<Icon name={properties.name} /> // build error
```

## Lazy loading

`LazyIcon` draws an icon chosen at run time, loading its module on first draw.

```tsx
import { LazyIcon } from "@destack/icon/lazy";

<LazyIcon name={file().icon} />;
```
