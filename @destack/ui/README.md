# @destack/ui

`@destack/ui/<component>` is shadcn/ui's components on Solid and StyleX (`Button`, `Dialog`, `Select`, `Sidebar`…) with Radix's parts and props, Base UI's `render`, and the prior art each component follows: cmdk's `Command`, vaul's `Drawer`, sonner's `toast`, input-otp's `InputOTP`, frimousse's `EmojiPicker`, react-day-picker's `Calendar`, react-resizable-panels' `Resizable`, TanStack Table's `DataTable` and TanStack Virtual's `createVirtualizer`, Every Layout's `Stack`, `Cluster`, `Grid`, `Center`, `Switcher` and `Cover`, and React Aria's collections, focus, selection and positioning.

```tsx
<Button variant="outline" xstyle={styles.wide} />; // shadcn's className as StyleX styles
<Button render={(part) => <a href="/notes" {...part} />} />; // Base UI's render as a function of the part's attributes
mergeProperties(part, caller); // Base UI's mergeProps
<TooltipContent side="right" sideOffset={8} />; // Radix's placement, through CSS anchor positioning
<Calendar mode="range" selected={trip()} onSelect={setTrip} disabled={isPast} />; // react-day-picker on PlainDate, where it takes Date
<InputOTP maxLength={6} onValueChange={setCode} />; // input-otp's onChange, which Solid keeps for the native change event
<Drawer activeSnapPoint={point()} onActiveSnapPointChange={setPoint} />; // vaul's setActiveSnapPoint
<Carousel loop autoplay={5000} setApi={setApi} />; // shadcn's Carousel on scroll snapping, without Embla's opts and plugins
<Stack space="4" splitAfter={2} />; // Every Layout's space as a spacing step
<SortableContainer id="todo" />; // dnd-kit's SortableContext, where Dice UI has one SortableContent
<Chart definition={defineChart({ marks: [barY(rows, { x: "day", y: "notes" })] })} />; // TanStack Charts in the theme's series colors
```

## Parts

Every part renders its native element with a `data-slot` and its state as `data-*` and applies `xstyle` after its own styles; element parts take `render`, and parts built on a native behaviour, such as `DialogContent` on `dialog`, keep their element.

```tsx
<Card render={(part) => <section {...part} />} />;
// <section data-slot="card">…</section>
```

## Building parts

`renderPart` builds an element part, and `useRender` builds one with its own attributes.

```tsx
export function CardFooter(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-footer", properties, styles.footer);
}
```

## Built-in text

Text the components write themselves renders in the reader's locale through `useLocale`, which also sets the arrow keys' direction.

```tsx
<Pagination />; // "Pagination", "Previous", "Next" in German under de-AT
```

## Content

`Content` renders a hast tree as elements and replaces the mapped tags with components, after MDX and Astro's `<Content />`.

```tsx
<Prose>
    <Content tree={post.tree} components={{ pre: Listing, figure: Figure }} />
</Prose>
```

## Code block

`CodeBlock` lays out, numbers and labels lines a highlighter wrote, coloring highlight.js token classes from the theme's roles.

```tsx
<CodeBlockContent isNumbered>
    {lines.map((line) => (
        <CodeBlockLine>
            <Content tree={line} />
        </CodeBlockLine>
    ))}
</CodeBlockContent>
```

## Glyph

`Glyph` draws an emoji, a named icon or an image at one of four sizes, hidden from assistive technology unless labelled.

```tsx
<Glyph glyph={{ emoji: "📘" }} />;
<Glyph glyph={{ icon: "rocket-launch" }} size="lg" label="Launch" />;
```

## Schema form

`SchemaForm` renders a field per property of a JSON Schema, and `renderField` replaces the control of a property.

```tsx
<SchemaForm
    schema={{
        type: "object",
        properties: {
            title: { type: "string", minLength: 1 },
            due: { type: "string", format: "date-time" }, // a DatePicker and a TimeField
            tags: { type: "array", items: { enum: ["home", "work"] } }, // a checkbox per value
            repeat: { oneOf: [daily, weekly] }, // a choice of kind, then its fields
        },
        required: ["title"],
    }}
    renderField={(field, held) => (field.name === "ownerId" ? <OwnerPicker {...held} /> : undefined)}
    onSubmit={file}
/>
```

## Color swatch picker

`ColorSwatchPicker` chooses one of the theme's accent presets from a list box of swatches.

```tsx
<ColorSwatchPicker name="color" value={color()} onValueChange={setColor} aria-label="Color" />
```

## Chart

`Chart` draws its scene as SVG, readable before the script runs, or on a canvas, with arrow-key focus between points and a visually hidden `table` of its rows.

```tsx
<Chart label="Ten thousand response times" renderer="canvas" definition={definition} />;
import { binX } from "@destack/ui/chart/transform/bin"; // each TanStack Charts entry point
```

## Examples and scenarios

Each component declares an example per visible state with `defineExample` and a scenario per keyboard or focus behavior with `defineScenario`.

```tsx
export const tabsSelectWithArrowKeys = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "select-with-arrow-keys",
    description: "select tabs as the arrow keys move the focus, skipping disabled tabs and wrapping",
    given: { examples: [tabsHistoryUnavailable] },
    when: [{ action: "focus", target: { role: "tab", name: "Edit" } }, { action: "press", key: "ArrowRight" }],
    then: {
        observe: { selected: { kind: "name", target: { role: "tab", selected: true } } },
        each: [{ selected: "Edit" }, { selected: "Preview" }],
    },
});
```

## Credits

The components follow the work of these projects.

- [shadcn/ui](https://github.com/shadcn-ui/ui) for the APIs and styles, MIT licensed, copyright (c) 2023 shadcn.
- [React Aria](https://github.com/adobe/react-spectrum) for collections, focus, selection and positioning, Apache 2.0 licensed, copyright 2020 Adobe.
- [Every Layout](https://every-layout.dev) by Heydon Pickering and Andy Bell for the layout primitives.
