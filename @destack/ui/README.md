# @destack/ui

Build views from accessible Solid components styled with StyleX and themed by the person's theme.

## Overview

Each component lives at `@destack/ui/<component>`, follows shadcn/ui's parts and props, and renders native elements styled by the theme's tokens.

- **Component**: a set of parts such as `Dialog`, `DialogTrigger` and `DialogContent`, after shadcn/ui and Radix, built on Solid and StyleX.
- **Part**: one rendered element with a `data-slot` and its state as `data-*`; `xstyle` adds styles after its own, and `render` swaps its element, as Base UI's does.
- **Layout**: Every Layout's `Stack`, `Cluster`, `Grid`, `Center`, `Switcher` and `Cover`, spaced in the theme's steps.
- **Collection**: React Aria's collections, focus, selection and positioning, under lists, menus, trees and tables.

```tsx
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { toast } from "@destack/ui/toast";

const styles = style.create({ wide: { width: "100%" } });

const Save = () => (
    <Button variant="outline" xstyle={styles.wide} onClick={() => toast("Saved")}>
        Save
    </Button>
);
```

## Examples

### Build a part

`renderPart` builds an element part with its slot, its styles and the caller's `render` and `xstyle`.

```ts
import * as style from "@destack/style";
import { renderPart } from "@destack/ui/part";

const styles = style.create({ footer: { display: "flex", gap: "0.5rem" } });

export function CardFooter(properties: { readonly children?: string }) {
    return renderPart("div", "card-footer", properties, styles.footer);
}
```

### Import component stylesheets

`Prose`, `CodeBlock`, `Stack` and `Switcher` style their descendants through a stylesheet each, which a page imports once.

```css
@import "@destack/ui/prose.css";
@import "@destack/ui/code-block.css";
```

## Comparison

### shadcn/ui

Components, parts and props follow shadcn/ui and Radix, and each component follows its own prior art, such as cmdk's `Command` or sonner's `toast`; three things differ.

- Components are Solid components styled with StyleX, where shadcn/ui copies React components styled with Tailwind.
- Positioning runs on CSS anchor positioning, and dialogs, popovers and drawers run on their native elements.
- Text the components write themselves renders in the reader's locale.
