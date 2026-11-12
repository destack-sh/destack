# @destack/ui

Build views from accessible Solid components styled with StyleX and themed by the person's theme.

## Overview

`@destack/ui` is shadcn/ui for Solid: accessible components, layouts after Every Layout, and the collections and positioning they are built from.

Views use components:

- Components, such as `Dialog`, `Select` and `DataTable`: sets of parts, one per entry at `@destack/ui/<component>`.
- Parts, such as `DialogTrigger`: one element each, with a `data-slot`, `xstyle` and `render`.

Pages lay them out:

- `Stack`, `Cluster`, `Grid`, `Center`, `Switcher` and `Cover`: Every Layout's layouts, spaced in the theme's steps.

Components build on:

- `renderPart`: builds a part from an element, its slot and styles.
- Collections, focus, selection and positioning: after React Aria, under lists, menus, trees and tables.

Style a button and show a toast:

```tsx
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { toast } from "@destack/ui/toast";

// a full-width style
const styles = style.create({ wide: { width: "100%" } });

// save, then confirm with a toast
export const Save = () => (
    <Button variant="outline" xstyle={styles.wide} onClick={() => toast("Saved")}>
        Save
    </Button>
);
```

## Reference

### Entries

Each entry is `@destack/ui/<entry>`.

| Group                  | Entries                                                                                                                                                                                                                                                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Forms                  | `autocomplete`, `button`, `button-group`, `calendar`, `checkbox`, `color-swatch-picker`, `combobox`, `date-field`, `date-picker`, `emoji-picker`, `field`, `icon-picker`, `input`, `input-group`, `input-otp`, `label`, `radio-group`, `schema-form`, `select`, `slider`, `switch`, `textarea`, `time-field`, `toggle`, `toggle-group`, `verification` |
| Overlays               | `alert-dialog`, `context-menu`, `dialog`, `drawer`, `dropdown-menu`, `hover-card`, `menubar`, `popover`, `sheet`, `toast`, `tooltip`                                                                                                                                                                                                                   |
| Navigation             | `breadcrumb`, `command`, `navigation-menu`, `pagination`, `sidebar`, `tabs`                                                                                                                                                                                                                                                                            |
| Data                   | `avatar`, `badge`, `card`, `carousel`, `chart`, `code-block`, `data-table`, `grid-list`, `item`, `kbd`, `list-box`, `table`, `tree`                                                                                                                                                                                                                    |
| Feedback and structure | `accordion`, `alert`, `aspect-ratio`, `collapsible`, `empty`, `progress`, `prose`, `resizable`, `scroll-area`, `separator`, `skeleton`, `spinner`, `visually-hidden`                                                                                                                                                                                   |
| Account                | `consent`, `sign-in`, `sign-up`                                                                                                                                                                                                                                                                                                                        |
| Layout                 | `stack`, `cluster`, `grid`, `center`, `switcher`, `cover`, `content`                                                                                                                                                                                                                                                                                   |
| Building blocks        | `part`, `collection`, `focus`, `position`, `selection`, `sortable`, `toggle-state`, `virtualizer`, `glyph`                                                                                                                                                                                                                                             |

### Parts

A part renders one element with `data-slot` and its state as `data-*` attributes.
`xstyle` adds styles after its own, and `render` swaps its element, as in Base UI.

`Prose`, `CodeBlock`, `Stack` and `Switcher` style their descendants, through `@destack/ui/prose.css`, `code-block.css`, `stack.css` and `switcher.css`.

## Examples

### Rename in a dialog

A dialog's parts follow shadcn/ui, with a field inside and close buttons that submit or cancel.

```tsx
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@destack/ui/dialog";
import { Field, FieldLabel } from "@destack/ui/field";
import { Input } from "@destack/ui/input";

/** Ask for a note's new title in a dialog. */
export const RenameDialog = () => (
    <Dialog>
        <DialogTrigger variant="outline">Rename</DialogTrigger>
        <DialogContent>
            {/* Header */}
            <DialogHeader>
                <DialogTitle>Rename note</DialogTitle>
                <DialogDescription>
                    The new title shows everywhere the note is shared.
                </DialogDescription>
            </DialogHeader>

            {/* Title */}
            <Field>
                <FieldLabel>Title</FieldLabel>
                <Input name="title" value="Groceries" />
            </Field>

            {/* Actions */}
            <DialogFooter>
                <DialogClose variant="outline">Cancel</DialogClose>
                <DialogClose type="submit">Save</DialogClose>
            </DialogFooter>
        </DialogContent>
    </Dialog>
);
```

### Pick from a select

A select groups its items, with a label per group.

```tsx
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from "@destack/ui/select";

/** Choose how notes sort. */
export const SortSelect = () => (
    <Select name="sort" aria-label="Sort by">
        <SelectTrigger>
            <SelectValue />
        </SelectTrigger>
        <SelectContent>
            {/* Dates */}
            <SelectGroup>
                <SelectLabel>Date</SelectLabel>
                <SelectItem value="updated">Last edited</SelectItem>
                <SelectItem value="created">Created</SelectItem>
            </SelectGroup>

            {/* Title */}
            <SelectSeparator />
            <SelectItem value="title">Title</SelectItem>
        </SelectContent>
    </Select>
);
```

### Generate a form from a schema

`SchemaForm` renders a field per property of a JSON Schema, such as a method's input.

```tsx
import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import { SchemaForm } from "@destack/ui/schema-form";

// a task's input, with titles shown as labels
const task = schema.object({
    title: schema.string().min(1).meta({ title: "Title" }),
    priority: schema.enum(["low", "normal", "high"]).meta({ title: "Priority" }).exactOptional(),
});

/** File a task from a form generated from its input. */
export const TaskForm = (properties: { readonly file: (input: unknown) => void }) => (
    <SchemaForm
        schema={JsonValue.object(toJsonSchema(task))}
        submit="File task"
        onSubmit={properties.file}
    />
);
```

### Build a part

`renderPart` takes an element, its slot, the caller's properties and its styles.

```ts
import * as style from "@destack/style";
import { renderPart } from "@destack/ui/part";

// a footer's own styles
const styles = style.create({ footer: { display: "flex", gap: "0.5rem" } });

export function CardFooter(properties: { readonly children?: string }) {
    return renderPart("div", "card-footer", properties, styles.footer);
}
```

### Import component stylesheets

A page imports each descendant stylesheet once.

```css
@import "@destack/ui/prose.css";
@import "@destack/ui/code-block.css";
```

## Comparison

### shadcn/ui

Components, parts and props follow shadcn/ui and Radix, and some follow their own prior art, such as cmdk's `Command` and sonner's `toast`.

| shadcn/ui               | `@destack/ui`                 | Difference                                         |
| ----------------------- | ----------------------------- | -------------------------------------------------- |
| copied React components | Solid components in a package | Styled with StyleX instead of Tailwind.            |
| Radix's Popper          | CSS anchor positioning        | Dialogs, popovers and drawers are native elements. |
| English strings         | the reader's locale           | Component text is translated.                      |
