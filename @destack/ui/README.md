# @destack/ui

Build interfaces from Solid and StyleX components on native elements and the WAI-ARIA patterns, styled by the theme's roles.

## Button

`Button` renders a native button in six variants and eight sizes, `render` puts its attributes on another element such as a router's link, and `buttonStyle` gives any element the same look.

```tsx
import { Button, buttonStyle } from "@destack/ui/button";

<Button type="submit">Save</Button>;
<Button variant="destructive" size="icon" aria-label="Delete note">
    <Icon name="trash" />
</Button>;
<Button loading>Save</Button>; // aria-busy="true" disabled, with a spinner
<Button variant="outline" render={(part) => <a href="/notes" {...part} />}>
    Notes
</Button>;
// <a href="/notes" data-slot="button" data-variant="outline" data-size="default">Notes</a>
```

## Button group

`ButtonGroup` joins buttons, inputs and selects into one control, flattening the corners where they meet.

```tsx
import { ButtonGroup, ButtonGroupSeparator } from "@destack/ui/button-group";

<ButtonGroup aria-label="Save">
    <Button>Save</Button>
    <ButtonGroupSeparator />
    <Button size="icon" aria-label="More ways to save">
        <Icon name="caret-down" />
    </Button>
</ButtonGroup>;
```

## Badge

`Badge` renders a short label in six variants, and `badgeStyle` gives a link the same look.

```tsx
import { Badge, badgeStyle } from "@destack/ui/badge";

<Badge variant="secondary">Draft</Badge>;

<a href="/tags/draft" {...style.attrs(badgeStyle({ variant: "secondary" }))}>
    Draft
</a>;
```

## Kbd

`Kbd` renders a key, and `KbdGroup` the keys of one shortcut.

```tsx
import { Kbd, KbdGroup } from "@destack/ui/kbd";

<KbdGroup>
    <Kbd>⌘</Kbd>
    <Kbd>K</Kbd>
</KbdGroup>;
```

## Spinner

`Spinner` announces loading as a status around a turning icon.

```tsx
import { Spinner } from "@destack/ui/spinner";

<Button disabled>
    <Spinner aria-label="Saving" />
    Save
</Button>;
```

## Skeleton

`Skeleton` renders a pulsing placeholder sized by its `style`.

```tsx
import { Skeleton } from "@destack/ui/skeleton";

const styles = style.create({ line: { width: "100%", height: space[4] } });
<Skeleton style={styles.line} />;
```

## Separator

`Separator` draws a line, hidden from assistive technology unless `decorative` is false.

```tsx
import { Separator } from "@destack/ui/separator";

<Separator />; // role="none"
<Separator orientation="vertical" decorative={false} />; // role="separator" aria-orientation="vertical"
```

## Visually hidden

`VisuallyHidden` keeps content out of sight for assistive technology to read, such as an icon button's name, and `visuallyHiddenStyle` gives any element the same styles.

```tsx
import { VisuallyHidden, visuallyHiddenStyle } from "@destack/ui/visually-hidden";

<Button variant="outline" size="icon">
    <Icon name="trash" />
    <VisuallyHidden>Delete note</VisuallyHidden>
</Button>;
<table {...style.attrs(visuallyHiddenStyle())}>...</table>;
```

## Aspect ratio

`AspectRatio` keeps a box at a width-to-height ratio as its width changes.

```tsx
import { AspectRatio } from "@destack/ui/aspect-ratio";

<AspectRatio ratio={16 / 9}>
    <img src="/photos/alfama.jpg" alt="Alfama at dusk" />
</AspectRatio>;
```

## Cover image

`CoverImage` shows an image as a banner centred on its position, which a drag moves while its owner listens, as do the arrow keys, Page Up, Page Down, Home and End on the vertical slider over it.

```tsx
import { CoverImage } from "@destack/ui/cover-image";

<CoverImage
    source="/covers/mountains.jpg"
    position={cover().position}
    onPositionChange={setPosition}
    label="Reposition the cover"
/>; // <input type="range" aria-orientation="vertical" value="33" aria-valuetext="33%">
```

## Avatar

`Avatar` shows a round picture, or its fallback until the picture loads or after it fails.

```tsx
import { Avatar, AvatarFallback, AvatarGroup, AvatarImage } from "@destack/ui/avatar";

<AvatarGroup aria-label="Shared with">
    <Avatar>
        <AvatarImage src="/people/ada.jpg" alt="Ada Lovelace" />
        <AvatarFallback>AL</AvatarFallback>
    </Avatar>
    <AvatarGroupCount>+4</AvatarGroupCount>
</AvatarGroup>;
```

## Glyph

`Glyph` draws an emoji, a named icon or an image at one of four sizes, hidden from assistive technology unless labelled.

```tsx
import { Glyph } from "@destack/ui/glyph";

<Glyph glyph={{ emoji: "📘" }} />;
<Glyph glyph={{ icon: "rocket-launch" }} size="lg" label="Launch" />;
<Glyph glyph={{ source: "/files/cover.webp" }} size="xl" />;
```

## Card

`Card` renders a raised surface with a header, content and footer.

```tsx
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from "@destack/ui/card";

<Card>
    <CardHeader>
        <CardTitle>Storage</CardTitle>
        <CardDescription>2 of 5 GB used</CardDescription>
        <CardAction>
            <Button variant="outline" size="sm">
                Upgrade
            </Button>
        </CardAction>
    </CardHeader>
    <CardContent>Notes take 1.2 GB.</CardContent>
</Card>;
```

## Alert

`Alert` announces a message that calls for attention, with an optional leading icon.

```tsx
import { Alert, AlertDescription, AlertTitle } from "@destack/ui/alert";

<Alert variant="destructive">
    <Icon name="warning-circle" />
    <AlertTitle>Sync paused</AlertTitle>
    <AlertDescription>Free up space or upgrade to keep syncing.</AlertDescription>
</Alert>;
```

## Empty

`Empty` shows what a list or page holds when it has nothing yet, with the actions that fill it.

```tsx
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "@destack/ui/empty";

<Empty>
    <EmptyHeader>
        <EmptyMedia variant="icon">
            <Icon name="notebook" />
        </EmptyMedia>
        <EmptyTitle>No notebooks yet</EmptyTitle>
        <EmptyDescription>Notebooks keep related notes together.</EmptyDescription>
    </EmptyHeader>
    <EmptyContent>
        <Button>Create a notebook</Button>
    </EmptyContent>
</Empty>;
```

## Prose

`Prose` styles rendered HTML, such as Markdown converted to GitHub's HTML, by its semantic elements, from theme tokens.

```tsx
import { Prose } from "@destack/ui/prose";

<Prose innerHTML={post.html} />; // headings, lists, tables, code, quotes, figures, footnotes and alerts
```

## Item

`Item` lays out media, content and actions in a row, listed as an entry inside an `ItemGroup`.

```tsx
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemMedia,
    ItemTitle,
    itemStyle,
} from "@destack/ui/item";

<ItemGroup aria-label="Recent notes">
    <Item>
        <ItemMedia variant="icon">
            <Icon name="notebook" />
        </ItemMedia>
        <ItemContent>
            <ItemTitle>Groceries</ItemTitle>
            <ItemDescription>Oat milk, lemons, rye bread</ItemDescription>
        </ItemContent>
        <ItemActions>
            <Button variant="outline" size="sm">
                Archive
            </Button>
        </ItemActions>
    </Item>
    <a href="/notes/trip" role="listitem" {...style.attrs(itemStyle({ size: "sm" }))}>
        Trip to Lisbon
    </a>
</ItemGroup>;
```

## Field

`Field` connects its label, descriptions, errors and state to its control, binds an owner's `value`, and lays out groups of fields.

```tsx
import {
    Field,
    FieldContent,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSeparator,
    FieldSet,
    FieldTitle,
} from "@destack/ui/field";

<Field invalid={error() !== undefined}>
    <FieldLabel>Name</FieldLabel>
    <Input />
    <FieldDescription>Shown to everyone you share the notebook with.</FieldDescription>
    <FieldError errors={[{ message: error() }]} />
</Field>;
// <label for="cl-0">, <input id="cl-0" aria-describedby="cl-1 cl-2" aria-invalid="true">,
// <p id="cl-1">, <div id="cl-2" role="alert">enter a name</div>

<Field
    value={{ text: () => title(), isChecked: () => false, commit: (raw) => rename(String(raw)) }}
>
    <FieldLabel>Title</FieldLabel>
    <Input />
</Field>;

<FieldSet>
    <FieldLegend>Notifications</FieldLegend>
    <FieldGroup>
        <Field orientation="responsive">
            <FieldContent>
                <FieldTitle>Mentions</FieldTitle>
                <FieldDescription>When someone mentions you in a note.</FieldDescription>
            </FieldContent>
            <Button variant="outline">Mute</Button>
        </Field>
        <FieldSeparator>or</FieldSeparator>
    </FieldGroup>
</FieldSet>;
```

## Schema form

`SchemaForm` renders a field per property of a JSON Schema and its local references, starting from each property's `default`, refuses values the schema refuses in the person's language, and renders a property through `renderField` when it returns a control.

```tsx
import { SchemaForm, schemaFields } from "@destack/ui/schema-form";

<SchemaForm
    schema={{
        type: "object",
        properties: {
            title: { type: "string", minLength: 1 },
            day: { type: "string", format: "date" }, // a DatePicker writing "2026-10-07"
            starts: { type: "string", format: "time" }, // a TimeField writing the UTC time "07:30:00Z"
            due: { type: "string", format: "date-time" }, // a DatePicker and a TimeField writing "2026-10-07T07:30:00.000Z"
            estimate: { type: "integer", minimum: 1, default: 1 }, // a number input with min and step
            tags: { type: "array", items: { enum: ["home", "work"] } }, // a checkbox per value
            steps: { type: "array", items: { type: "string" } }, // rows to add and remove
            place: { type: "object", properties: { city: { type: "string" } } }, // a fieldset
            repeat: { oneOf: [daily, weekly] }, // a choice of kind, then its fields
        },
        required: ["title"],
    }}
    submit="File task"
    onSubmit={(value) => file(value)} // { title: "Milk", estimate: 1 } once every field is kept
/>;
schemaFields(input).length; // 0 for a command that takes nothing

<SchemaForm
    schema={{
        $defs: {
            node: {
                type: "object",
                properties: {
                    name: { type: "string" },
                    children: { type: "array", items: { $ref: "#/$defs/node" } },
                },
                required: ["name"],
            },
        },
        type: "object",
        properties: { tree: { $ref: "#/$defs/node" } },
        required: ["tree"],
    }}
    onSubmit={save}
/>;
schemaFields({ properties: { owner: { $ref: "#/$defs/person" } } }); // TypeError: schema reference #/$defs/person does not resolve

<SchemaForm
    schema={input}
    renderField={(field, held) =>
        field.name === "ownerId" ? (
            <OwnerPicker value={held.value()} onChange={held.set} />
        ) : undefined
    }
    onSubmit={file}
/>;
```

## Label

`Label` renders a native label.

```tsx
import { Label } from "@destack/ui/label";

<Label for="email">Email</Label>;
```

## Input

`Input` renders a native input.

```tsx
import { Input } from "@destack/ui/input";

<Input type="search" placeholder="Search notes" aria-label="Search notes" />;
<Input aria-invalid="true" />; // the destructive border and ring
```

## Input group

`InputGroup` frames an input or textarea together with its icons, text and buttons.

```tsx
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
} from "@destack/ui/input-group";

<InputGroup>
    <InputGroupInput type="search" aria-label="Search notes" />
    <InputGroupAddon>
        <Icon name="magnifying-glass" />
    </InputGroupAddon>
    <InputGroupAddon align="inline-end">
        <InputGroupButton size="icon-xs" aria-label="Clear search">
            <Icon name="x" />
        </InputGroupButton>
    </InputGroupAddon>
</InputGroup>;
```

## Input OTP

`InputOTP` shows a one-time code one character per slot over a native input that takes typing, pasting and the platform's code autofill.

```tsx
import { InputOTP, InputOTPGroup, InputOTPSeparator, InputOTPSlot } from "@destack/ui/input-otp";

<InputOTP maxLength={6} onComplete={verify}>
    <InputOTPGroup>
        <InputOTPSlot index={0} />
        <InputOTPSlot index={1} />
        <InputOTPSlot index={2} />
    </InputOTPGroup>
    <InputOTPSeparator />
    <InputOTPGroup>
        <InputOTPSlot index={3} />
        <InputOTPSlot index={4} />
        <InputOTPSlot index={5} />
    </InputOTPGroup>
</InputOTP>;
```

## Textarea

`Textarea` renders a native textarea that grows with its content.

```tsx
import { Textarea } from "@destack/ui/textarea";

<Textarea name="comment" placeholder="Add a comment" aria-label="Comment" />;
```

## Select

`Select` renders a native `<select>` whose button, picker and options take the theme where the browser supports customizable selects, with a `placeholder`, and a list box when `multiple`.

```tsx
import { Select, SelectItem, SelectTrigger, SelectValue } from "@destack/ui/select";

<Select aria-label="Sort by" placeholder="Sort by…" onValueChange={setSort}>
    <SelectTrigger>
        <SelectValue />
    </SelectTrigger>
    <SelectItem value="updated">Last edited</SelectItem>
    <SelectItem value="title">Title</SelectItem>
</Select>;
<Select aria-label="Tags" multiple value={tags()} onValueChange={setTags}>
    <SelectItem value="travel">Travel</SelectItem>
    <SelectItem value="food">Food</SelectItem>
</Select>;
```

## Checkbox

`Checkbox` renders a button exposed as a checkbox around its `CheckboxIndicator`, which a click or Space toggles, and a hidden native checkbox carries its state into a form and back from its reset.

```tsx
import { Checkbox, CheckboxIndicator } from "@destack/ui/checkbox";

<Checkbox name="terms" required defaultChecked />;
<Checkbox
    aria-label="Select all"
    checked={isAll()}
    indeterminate={isSome()}
    onCheckedChange={selectAll}
>
    <CheckboxIndicator xstyle={styles.mark} />
</Checkbox>;
// aria-checked="mixed" data-state="indeterminate"
```

## Radio group

`RadioGroup` holds one value for its `RadioGroupItem` radios, which the arrow keys move between and check with one tab stop, each around its `RadioGroupIndicator`, and hidden native radios carry the value into a form.

```tsx
import { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "@destack/ui/radio-group";

<RadioGroup name="density" defaultValue="regular" orientation="horizontal" required>
    <RadioGroupItem value="compact" aria-label="Compact" />
    <RadioGroupItem value="regular" aria-label="Regular">
        <RadioGroupIndicator xstyle={styles.dot} />
    </RadioGroupItem>
</RadioGroup>;
```

## Swatch picker

`SwatchPicker` chooses one of the theme's accent presets, every colorful one or its `presets`, as a list box of colored `SwatchPickerItem` swatches in rows of `columns`, whose arrow keys choose as they move, and a hidden input carries the choice into a form.

```tsx
import { SwatchPicker, SwatchPickerItem } from "@destack/ui/swatch-picker";

<SwatchPicker name="color" value={color()} onValueChange={setColor} aria-label="Color" />;
<SwatchPicker defaultValue="teal" columns={2} aria-label="Label color">
    <SwatchPickerItem value="teal" />
    <SwatchPickerItem value="plum" />
</SwatchPicker>;
// role="listbox" with role="option" aria-selected="true" on the chosen swatch
```

## Switch

`Switch` renders a button exposed as a switch around its `SwitchThumb`, and a hidden native checkbox carries its state into a form.

```tsx
import { Switch, SwitchThumb } from "@destack/ui/switch";

<Switch aria-label="Notifications" checked={isOn()} onCheckedChange={setOn} />;
<Switch name="airplane-mode" defaultChecked>
    <SwitchThumb xstyle={styles.thumb} />
</Switch>;
// role="switch" aria-checked="true" data-state="checked"
```

## Slider

`Slider` holds one value, or two for a range, for its `SliderTrack`, `SliderRange` and a `SliderThumb` per value, each thumb a native range input.

```tsx
import { Slider, SliderRange, SliderThumb, SliderTrack } from "@destack/ui/slider";

<Slider min={12} max={24} defaultValue={[16]} aria-label="Font size" />;
<Slider
    max={500}
    step={10}
    value={price()}
    onValueChange={setPrice}
    marks={[0, 250, 500]}
    aria-label="Price"
>
    <SliderTrack>
        <SliderRange xstyle={styles.fill} />
    </SliderTrack>
    <SliderThumb />
    <SliderThumb />
</Slider>;
// thumbs named "Minimum" and "Maximum"
```

## Progress

`Progress` reports work done as a progress bar filled by its `ProgressIndicator`, sweeping while it has no value.

```tsx
import { Progress, ProgressIndicator } from "@destack/ui/progress";

<Progress value={30} aria-label="Upload" />;
// aria-valuetext="30%" data-state="loading"
<Progress aria-label="Sync">
    <ProgressIndicator xstyle={styles.bar} />
</Progress>;
// data-state="indeterminate"
```

## Toggle

`Toggle` renders a button that turns on and off, and `toggleStyle` gives other elements its look.

```tsx
import { Toggle } from "@destack/ui/toggle";

<Toggle variant="outline" aria-label="Bold" onPressedChange={setBold}>
    <Icon name="text-b" />
</Toggle>;
```

## Toggle group

`ToggleGroup` holds one pressed value or several, with one tab stop.

```tsx
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";

<ToggleGroup defaultValue="left" aria-label="Alignment">
    <ToggleGroupItem value="left">Left</ToggleGroupItem>
    <ToggleGroupItem value="center">Center</ToggleGroupItem>
</ToggleGroup>;
<ToggleGroup multiple value={["bold"]} onValueChange={setMarks} aria-label="Style">
    …
</ToggleGroup>;
```

## Collection

`Collection` holds a component's items in sections, by key and by place, waiting for a `Load` of its items and throwing a failed load to the nearest error boundary; `CollectionBuilder` collects the items child components add as they mount, in document order.

```ts
import { Collection, CollectionBuilder, Load } from "@destack/ui/collection";

const notes = new Load(fetchNotes());
const collection = new Collection({
    load: notes,
    sections: () => [{ key: "notes", label: "Notes", items: notes.value() }],
    key: (note) => note.id,
    text: (note) => note.title,
});
collection.state(); // "loading", "loaded" or "failed"
collection.keys(); // every key in order
const items = new CollectionBuilder((item: Item) => item.element()); // items.add(item) as each mounts
```

## Focus

`Focus` keeps a collection's one focused key, moved by a keyboard delegate, as a roving tab stop or behind `aria-activedescendant`; `ListDelegate` and `GridDelegate` move along a list or across rows of a column count, and `ListState` joins them for components whose items mount as children.

```ts
import { Focus, GridDelegate, ListDelegate, ListState } from "@destack/ui/focus";

const focus = new Focus({
    delegate: new ListDelegate(collection, { orientation: "vertical", isLooping: true, isTypeahead: true }),
    mode: "roving",
});
focus.move(event, locale.direction); // the key it lands on, undefined for a key it leaves alone
focus.isActive(key); // reruns only for the key the focus leaves and the one it reaches
new GridDelegate(collection, () => 9); // the arrow keys across rows of nine
new ListState({ orientation: "horizontal", isLooping: true, isTypeahead: false }); // tabs, radio groups, menus
```

## Selection

`Selection` keeps a collection's selected values, one or several, controlled by its owner or its own.

```ts
import { Selection } from "@destack/ui/selection";

const tags = new Selection({ multiple: true, defaultValue: ["travel"] });
tags.toggle("work");
tags.values(); // ["travel", "work"]
new Selection({ value: notebook(), onValueChange: setNotebook }); // controlled, one value
```

## Position

`Position` places an overlay beside its anchor or at a point, through CSS anchor positioning where the browser has it and by measuring where it lacks it.

```ts
import { Position } from "@destack/ui/position";

Position.beside("bottom", "start"); // StyleX styles placing an overlay below its anchor
Position.at({ x: event.clientX, y: event.clientY }); // a context menu at the pointer
const stop = Position.place(content, anchor); // follow scrolls and resizes without anchor positioning
```

## Toggle state

`ToggleState` keeps a control's on or off state, controlled or its own, and `ToggleInput` carries it into the control's form, following a form reset.

```tsx
import { ToggleInput, ToggleState } from "@destack/ui/toggle-state";

const state = new ToggleState({ defaultChecked: true, onCheckedChange: save });
<ToggleInput name="notify" checked={state.isChecked()} disabled={false} onReset={() => state.reset()} />;
```

## Virtualizer

`createVirtualizer` renders only the rows of a long list in and near its scrolling element, measuring each as it renders.

```ts
import { createVirtualizer } from "@destack/ui/virtualizer";

const rows = createVirtualizer({ count: () => notes().length, itemHeight: 36, scrollElement: () => viewport });
rows.items(); // the rows to render, each with its index and offset
rows.reveal(420); // scroll a row into view
```

## List box

`ListBox` offers `ListBoxItem` options in `ListBoxSection` sections, which the arrow keys, Home, End and typed letters move through in a list or a grid of `columns`, and Enter, Space or a click choose into its single or multiple `selection`.

```tsx
import {
    ListBox,
    ListBoxEmpty,
    ListBoxItem,
    ListBoxSection,
    ListBoxSeparator,
} from "@destack/ui/list-box";

<ListBox aria-label="Notebook" selection={{ value: notebook(), onValueChange: setNotebook }}>
    <ListBoxSection heading="Active">
        <ListBoxItem value="trips">Trips</ListBoxItem>
        <ListBoxItem value="work">Work</ListBoxItem>
    </ListBoxSection>
    <ListBoxSeparator />
    <ListBoxItem value="taxes" disabled>
        Taxes
    </ListBoxItem>
    <ListBoxEmpty>No notebooks</ListBoxEmpty>
</ListBox>;
<ListBox aria-label="Tags" selection={{ multiple: true, defaultValue: ["travel"] }}>
    …
</ListBox>;
<ListBox aria-label="Label" layout="grid" columns={3} selectionBehavior="replace" selection={{}}>
    …
</ListBox>; // the arrow keys move across rows and columns, choosing as they move
<ListBox aria-label="Actions" onAction={(option) => run(option.value())}>
    …
</ListBox>; // no selection: Enter and a click run the option's action
```

## Grid list

`GridList` lays items out by section in rows of a column count, rendering only the rows in its `GridListViewport`, which the arrow keys, Home and End move through and Enter or a click choose, between its `GridListLoading` and `GridListEmpty` notes.

```tsx
import {
    GridList,
    GridListControl,
    GridListEmpty,
    GridListProvider,
    GridListViewport,
} from "@destack/ui/grid-list";

const grid = new GridListControl({
    sections: () => sections(), // [{ key: "operators", label: "Operators", items: [...] }]
    key: (symbol) => symbol.id,
    text: (symbol) => symbol.name,
    columns: () => 9,
    onAction: insert,
});
<GridListProvider control={grid}>
    <GridListViewport>
        <GridListEmpty>No symbols</GridListEmpty>
        <GridList control={grid} aria-label="Symbols" label={(symbol) => symbol.name} />
    </GridListViewport>
</GridListProvider>;
// role="grid" aria-activedescendant names the focused role="gridcell"
```

## Autocomplete

`Autocomplete` holds a search whose `AutocompleteInput` filters the one list box or grid list inside by its options' values and keywords, and moves its focus from the search field with the arrow keys and Enter.

```tsx
import { Autocomplete, AutocompleteInput } from "@destack/ui/autocomplete";

<Autocomplete search={query()} onSearchChange={setQuery}>
    <AutocompleteInput aria-label="Fruit" />
    <ListBox aria-label="Fruits" onAction={(option) => pick(option.value())}>
        <ListBoxItem keywords={["tropical"]}>Mango</ListBoxItem>
        <ListBoxEmpty>No fruit found</ListBoxEmpty>
    </ListBox>
</Autocomplete>;
<Autocomplete shouldFilter={false}>…</Autocomplete>; // the owner filters, such as by loading matches
<Autocomplete filter={(value, search) => value.startsWith(search)}>…</Autocomplete>;
```

## Combobox

`Combobox` is an input, a popover and an autocomplete over a list box, which filters its options as the person types, after the APG combobox pattern.

```tsx
import {
    Combobox,
    ComboboxChip,
    ComboboxItemIndicator,
    ComboboxChipRemove,
    ComboboxChips,
    ComboboxContent,
    ComboboxCreate,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
    ComboboxLoading,
} from "@destack/ui/combobox";

<Combobox onValueChange={move}>
    <ComboboxInput aria-label="Notebook" />
    <ComboboxContent>
        <ComboboxEmpty>No notebook found</ComboboxEmpty>
        <ComboboxItem value="Trips">
            Trips
            <ComboboxItemIndicator />
        </ComboboxItem>
    </ComboboxContent>
</Combobox>;
<Combobox multiple value={tags()} onValueChange={setTags} onCreate={addTag}>
    <ComboboxChips />
    <ComboboxInput aria-label="Tags" />
    <ComboboxContent>
        <ComboboxItem value="travel">travel</ComboboxItem>
        <ComboboxCreate />
    </ComboboxContent>
</Combobox>;
<Combobox multiple defaultValue={["travel"]}>
    <ComboboxChips>
        <For each={tags()}>
            {(tag) => (
                <ComboboxChip value={tag}>
                    {tag} <ComboboxChipRemove />
                </ComboboxChip>
            )}
        </For>
    </ComboboxChips>
    <ComboboxInput aria-label="Tags" />
</Combobox>;
<Combobox shouldFilter={false} inputValue={query()} onInputValueChange={search}>
    <ComboboxInput aria-label="Person" />
    <ComboboxContent>
        <Show
            when={isLoading()}
            fallback={
                <For each={people()}>
                    {(person) => <ComboboxItem value={person.id}>{person.name}</ComboboxItem>}
                </For>
            }
        >
            <ComboboxLoading>Searching…</ComboboxLoading>
        </Show>
    </ComboboxContent>
</Combobox>;
```

## Command

`Command` is an autocomplete over a list box, which searches options by text and keywords that the arrow keys, Home and End highlight and Enter chooses, and `CommandDialog` shows it as a palette.

```tsx
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandGroupHeading,
    CommandInput,
    CommandItem,
    CommandList,
} from "@destack/ui/command";

<Command aria-label="Commands" value={highlighted()} onValueChange={setHighlighted} loop>
    <CommandInput placeholder="Type a command" />
    <CommandList>
        <CommandEmpty>No results</CommandEmpty>
        <CommandGroup heading="Notes">
            <CommandItem keywords={["trash"]} onSelect={remove}>
                Delete note
            </CommandItem>
        </CommandGroup>
    </CommandList>
</Command>;

<Command
    search={query()}
    onSearchChange={setQuery}
    filter={(value, search) => value.startsWith(search)}
>
    …
    <CommandGroup>
        <CommandGroupHeading xstyle={styles.heading}>Recent</CommandGroupHeading>
        <CommandItem forceMount>Help</CommandItem>
    </CommandGroup>
</Command>;
```

## Emoji picker

`EmojiPicker` is an autocomplete over a grid list of every emoji by category in a skin tone, around its `EmojiPickerSearch`, `EmojiPickerContent` and `EmojiPickerFooter`, loading their names and words in the reader's locale, else English, once it renders, and showing a failed load in the nearest `Errored`.

```tsx
import {
    EmojiPicker,
    EmojiPickerContent,
    EmojiPickerEmoji,
    EmojiPickerFooter,
    EmojiPickerSearch,
    EmojiPickerSkinTone,
    EmojiPickerSkinToneSelector,
} from "@destack/ui/emoji-picker";

<EmojiPicker columns={9} defaultTone={0} onPick={(entry) => react(entry.emoji)}>
    <EmojiPickerSearch />
    <EmojiPickerSkinTone />
    <EmojiPickerContent />
    <EmojiPickerFooter>
        <EmojiPickerSkinToneSelector />
    </EmojiPickerFooter>
</EmojiPicker>;
// the search takes typing while the emoji load, the content showing a status until they do
<EmojiPicker
    cell={(cell) => <EmojiPickerEmoji {...cell} xstyle={styles.cell} />}
    activeCell={(entry) => entry?.label}
    onPick={(entry) => react(entry.emoji)}
>
    <EmojiPickerSearch />
    <EmojiPickerContent />
    <EmojiPickerFooter />
</EmojiPicker>;
```

## Icon picker

`IconPicker` is an autocomplete over a grid list of the icon set's icons by category, around its `IconPickerSearch`, `IconPickerContent` and `IconPickerFooter`, finding them by name and tag.

```tsx
import {
    IconPicker,
    IconPickerContent,
    IconPickerFooter,
    IconPickerIcon,
    IconPickerSearch,
} from "@destack/ui/icon-picker";

<IconPicker cell={(cell) => <IconPickerIcon {...cell} xstyle={styles.cell} />} onPick={setIcon}>
    <IconPickerSearch />
    <IconPickerContent />
    <IconPickerFooter />
</IconPicker>;
// the footer shows the active icon and its name
```

## Sortable

`Sortable` lets a person reorder items in lists, across a board's columns and through a tree's levels by pointer or keyboard, announcing each step and reporting each move by the items it lands between.

```tsx
import { Sortable, SortableContainer, SortableHandle, SortableItem } from "@destack/ui/sortable";

<Sortable onMove={({ id, container, before, after }) => move(id, container, before, after)}>
    <SortableContainer id="todo" label="To do">
        <For each={todo()}>
            {(card) => (
                <SortableItem id={card.id} label={card.title}>
                    <SortableHandle /> {card.title}
                </SortableItem>
            )}
        </For>
    </SortableContainer>
    <SortableContainer id="done" label="Done">
        …
    </SortableContainer>
</Sortable>;

<Sortable nesting onMove={move}>
    <SortableContainer id="pages" label="Pages">
        <SortableItem id="guide">
            <SortableHandle /> Guide
            <SortableContainer id="guide">…</SortableContainer>
        </SortableItem>
    </SortableContainer>
</Sortable>;
```

## Dialog

`Dialog` opens a native modal `<dialog>`, which makes the page inert, closes on Escape and returns the focus.

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

<Dialog onOpenChange={(open) => track(open)}>
    <DialogTrigger variant="outline">Rename</DialogTrigger>
    <DialogContent>
        <DialogHeader>
            <DialogTitle>Rename note</DialogTitle>
            <DialogDescription>The new title shows everywhere.</DialogDescription>
        </DialogHeader>
        <DialogFooter>
            <DialogClose>Save</DialogClose>
        </DialogFooter>
    </DialogContent>
</Dialog>;
// <dialog aria-labelledby aria-describedby closedby="any">
<Dialog modal={false}>…</Dialog>; // shown with show()
```

## Alert dialog

`AlertDialog` asks to confirm an action and closes only from its buttons.

```tsx
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogTitle,
    AlertDialogTrigger,
} from "@destack/ui/alert-dialog";

<AlertDialog>
    <AlertDialogTrigger variant="destructive">Delete</AlertDialogTrigger>
    <AlertDialogContent>
        <AlertDialogTitle>Delete Trips?</AlertDialogTitle>
        <AlertDialogCancel>Cancel</AlertDialogCancel>
        <AlertDialogAction variant="destructive" onClick={remove}>
            Delete
        </AlertDialogAction>
    </AlertDialogContent>
</AlertDialog>;
```

## Sheet

`Sheet` slides a dialog in from an edge.

```tsx
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@destack/ui/sheet";

<Sheet>
    <SheetTrigger variant="outline">Filters</SheetTrigger>
    <SheetContent side="left">
        <SheetTitle>Filters</SheetTitle>
    </SheetContent>
</Sheet>;
```

## Drawer

`Drawer` slides a dialog in from its direction, with a handle from the bottom, which a swipe toward its edge closes and a drag settles at its snap points.

```tsx
import {
    Drawer,
    DrawerContent,
    DrawerHandle,
    DrawerTitle,
    DrawerTrigger,
} from "@destack/ui/drawer";

<Drawer direction="bottom">
    <DrawerTrigger>Share</DrawerTrigger>
    <DrawerContent>
        <DrawerTitle>Share Groceries</DrawerTitle>
    </DrawerContent>
</Drawer>;

<Drawer snapPoints={[0.5, "320px", 1]} activeSnapPoint={point()} onSnapPointChange={setPoint}>
    <DrawerContent showHandle={false}>
        <DrawerHandle xstyle={styles.handle} />…
    </DrawerContent>
</Drawer>; // data-snap-point="2"
```

## Popover

`Popover` opens its content in the top layer through the Popover API, anchored to its trigger, or as a modal dialog.

```tsx
import {
    Popover,
    PopoverAnchor,
    PopoverClose,
    PopoverContent,
    PopoverDescription,
    PopoverHeader,
    PopoverTitle,
    PopoverTrigger,
} from "@destack/ui/popover";

<Popover>
    <PopoverTrigger variant="outline">Size</PopoverTrigger>
    <PopoverContent side="bottom" align="start">
        …
    </PopoverContent>
</Popover>;

<Popover modal>
    <PopoverTrigger variant="outline">Rename</PopoverTrigger>
    <PopoverContent>…</PopoverContent>
</Popover>; // a <dialog> shown with showModal()

<Popover>
    <PopoverAnchor>{row}</PopoverAnchor>
    <PopoverTrigger variant="ghost">Edit</PopoverTrigger>
    <PopoverContent>
        <PopoverHeader>
            <PopoverTitle>Rename</PopoverTitle>
            <PopoverDescription>Names show in every list.</PopoverDescription>
        </PopoverHeader>
        <PopoverClose>Done</PopoverClose>
    </PopoverContent>
</Popover>; // aria-labelledby="…-title" aria-describedby="…-description"
```

## Hover card

`HoverCard` previews a link once the pointer or focus rests on it.

```tsx
import { HoverCard, HoverCardContent, HoverCardTrigger } from "@destack/ui/hover-card";

<HoverCard openDelay={700} closeDelay={300}>
    <HoverCardTrigger href="/people/ada">@ada</HoverCardTrigger>
    <HoverCardContent>Ada Lovelace</HoverCardContent>
</HoverCard>;
```

## Tooltip

`Tooltip` describes its trigger on focus or after a resting pointer, and stays while the pointer is on it.

```tsx
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@destack/ui/tooltip";

<Tooltip delayDuration={700}>
    <TooltipTrigger variant="ghost" size="icon" aria-label="Archive">
        <Icon name="archive" />
    </TooltipTrigger>
    <TooltipContent side="top">Archive note</TooltipContent>
</Tooltip>;
<TooltipProvider delayDuration={700} skipDelayDuration={300}>
    …
</TooltipProvider>;
```

## Toast

`toast` shows a message in the `Toaster`, a live region above dialogs that Alt+T focuses.

```tsx
import { Toaster, toast } from "@destack/ui/toast";

<Toaster position="bottom-right" duration={4000} />;
toast("Note archived", { action: { label: "Undo", onClick: restore } });
toast.promise(save(), { loading: "Saving", success: "Saved", error: "Saving failed" });
<Toaster position="top-center" richColors />;
```

## Dropdown menu

`DropdownMenu` opens a menu from its trigger after the APG menu button pattern, with checkbox and radio items and submenus.

```tsx
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";

<DropdownMenu open={isOpen()} onOpenChange={setOpen}>
    <DropdownMenuTrigger variant="outline">Note</DropdownMenuTrigger>
    <DropdownMenuContent>
        <DropdownMenuItem onSelect={rename}>Rename</DropdownMenuItem>
        <DropdownMenuItem variant="destructive">Delete</DropdownMenuItem>
    </DropdownMenuContent>
</DropdownMenu>;
```

## Context menu

`ContextMenu` opens the same menu at the pointer.

```tsx
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuTrigger,
} from "@destack/ui/context-menu";

<ContextMenu>
    <ContextMenuTrigger>Groceries</ContextMenuTrigger>
    <ContextMenuContent>
        <ContextMenuItem>Open</ContextMenuItem>
    </ContextMenuContent>
</ContextMenu>;
```

## Menubar

`Menubar` lines up menus after the APG menubar pattern.

```tsx
import {
    Menubar,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
    MenubarTrigger,
} from "@destack/ui/menubar";

<Menubar aria-label="Editor">
    <MenubarMenu>
        <MenubarTrigger>File</MenubarTrigger>
        <MenubarContent>
            <MenubarItem>New note</MenubarItem>
        </MenubarContent>
    </MenubarMenu>
</Menubar>;
```

## Tabs

`Tabs` switches between panels after the APG tabs pattern, on focus or on click.

```tsx
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@destack/ui/tabs";

<Tabs defaultValue="edit" activationMode="automatic">
    <TabsList aria-label="Note">
        <TabsTrigger value="edit">Edit</TabsTrigger>
        <TabsTrigger value="preview">Preview</TabsTrigger>
    </TabsList>
    <TabsContent value="edit">The editor</TabsContent>
    <TabsContent value="preview">The rendered note</TabsContent>
</Tabs>;
```

## Accordion

`Accordion` stacks native `<details>` disclosures, one open at a time in a `single` accordion, which arrow keys move between.

```tsx
import {
    Accordion,
    AccordionContent,
    AccordionItem,
    AccordionTrigger,
} from "@destack/ui/accordion";

<Accordion defaultValue="sharing">
    <AccordionItem value="sharing">
        <AccordionTrigger>Who can see my notes?</AccordionTrigger>
        <AccordionContent>Only the people you share a notebook with.</AccordionContent>
    </AccordionItem>
    <AccordionItem value="billing" disabled>
        <AccordionTrigger>How do I pay?</AccordionTrigger>
    </AccordionItem>
</Accordion>;
```

## Collapsible

`Collapsible` shows and hides its content in a native `<details>`.

```tsx
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@destack/ui/collapsible";

<Collapsible open={isOpen()} onOpenChange={setOpen}>
    <CollapsibleTrigger>travel and 3 more</CollapsibleTrigger>
    <CollapsibleContent>food, family, summer</CollapsibleContent>
</Collapsible>;
```

## Navigation menu

`NavigationMenu` discloses panels of links from its triggers after the APG disclosure navigation pattern, its open item's `value` controlled or its own.

```tsx
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuIndicator,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
    navigationMenuTriggerStyle,
} from "@destack/ui/navigation-menu";

<NavigationMenu aria-label="Main" value={open()} onValueChange={setOpen} delayDuration={200}>
    <NavigationMenuList>
        <NavigationMenuItem value="products">
            <NavigationMenuTrigger>Products</NavigationMenuTrigger>
            <NavigationMenuContent>
                <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
            </NavigationMenuContent>
        </NavigationMenuItem>
        <NavigationMenuItem>
            <NavigationMenuLink href="/pricing" active style={navigationMenuTriggerStyle()}>
                Pricing
            </NavigationMenuLink>
        </NavigationMenuItem>
        <NavigationMenuIndicator />
    </NavigationMenuList>
</NavigationMenu>;
<NavigationMenu aria-label="Main" viewport={false}>
    …
</NavigationMenu>;
```

## Breadcrumb

`Breadcrumb` renders the trail to the current page as a navigation landmark.

```tsx
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "@destack/ui/breadcrumb";

<Breadcrumb>
    <BreadcrumbList>
        <BreadcrumbItem>
            <BreadcrumbLink href="/notebooks">Notebooks</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
            <BreadcrumbPage>Lisbon</BreadcrumbPage>
        </BreadcrumbItem>
    </BreadcrumbList>
</Breadcrumb>;
```

## Pagination

`Pagination` renders links to pages as a navigation landmark.

```tsx
import {
    Pagination,
    PaginationContent,
    PaginationItem,
    PaginationLink,
    PaginationNext,
    PaginationPages,
    PaginationPrevious,
} from "@destack/ui/pagination";

<Pagination>
    <PaginationContent>
        <PaginationItem>
            <PaginationPrevious href="?page=1" />
        </PaginationItem>
        <PaginationItem>
            <PaginationLink href="?page=2" active>
                2
            </PaginationLink>
        </PaginationItem>
        <PaginationItem>
            <PaginationNext href="?page=3" />
        </PaginationItem>
    </PaginationContent>
</Pagination>;
<Pagination>
    <PaginationPages
        count={20}
        page={page()}
        onPageChange={setPage}
        href={(page) => `?page=${page}`}
    />
</Pagination>;
// 1 … 5 6 7 … 20
```

## Sidebar

`Sidebar` collapses off canvas or to icons on wide screens and opens as a sheet on narrow ones, remembered per viewer and toggled with Ctrl or ⌘ B.

```tsx
import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupLabel,
    SidebarInput,
    SidebarInset,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarProvider,
    SidebarTrigger,
} from "@destack/ui/sidebar";

<SidebarProvider>
    <Sidebar collapsible="icon" side="left" variant="sidebar">
        <SidebarContent>
            <SidebarInput type="search" aria-label="Filter notebooks" />
            <SidebarGroup>
                <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                <SidebarMenu>
                    <SidebarMenuItem>
                        <SidebarMenuButton href="/trips" active>
                            Trips
                        </SidebarMenuButton>
                    </SidebarMenuItem>
                </SidebarMenu>
            </SidebarGroup>
        </SidebarContent>
    </Sidebar>
    <SidebarInset>
        <SidebarTrigger />
    </SidebarInset>
</SidebarProvider>;
```

## Scroll area

`ScrollArea` scrolls its content with themed native scrollbars and takes the focus.

```tsx
import { ScrollArea } from "@destack/ui/scroll-area";

<ScrollArea aria-label="Tags" orientation="vertical" style={styles.tags}>
    …
</ScrollArea>;
```

## Resizable

`ResizablePanelGroup` splits panels by shares that its handles move, after the APG window splitter pattern.

```tsx
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@destack/ui/resizable";

<ResizablePanelGroup direction="horizontal">
    <ResizablePanel defaultSize={30} minSize={20}>
        Notebooks
    </ResizablePanel>
    <ResizableHandle withHandle aria-label="Resize the notebook list" />
    <ResizablePanel>Note</ResizablePanel>
</ResizablePanelGroup>;
<ResizablePanelGroup autoSaveId="notebook-split" onLayout={(layout) => save(layout)}>
    <ResizablePanel defaultSize={30} minSize={20} collapsible>
        Notebooks
    </ResizablePanel>
    <ResizableHandle />
    <ResizablePanel>Note</ResizablePanel>
</ResizablePanelGroup>;
```

## Carousel

`Carousel` scrolls its slides with CSS scroll snapping, after the APG carousel pattern.

```tsx
import {
    Carousel,
    CarouselContent,
    CarouselItem,
    CarouselDots,
    CarouselNext,
    CarouselPlay,
    CarouselPrevious,
} from "@destack/ui/carousel";

<Carousel aria-label="Photos">
    <CarouselContent>
        <CarouselItem>…</CarouselItem>
    </CarouselContent>
    <CarouselPrevious />
    <CarouselNext />
</Carousel>;
<Carousel aria-label="Photos" loop autoplay={5000}>
    <CarouselContent>…</CarouselContent>
    <CarouselPlay />
    <CarouselDots />
</Carousel>;
```

## Table

`Table` renders a native table that scrolls sideways.

```tsx
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";

<Table>
    <TableHeader>
        <TableRow>
            <TableHead scope="col">Invoice</TableHead>
        </TableRow>
    </TableHeader>
    <TableBody>
        <TableRow>
            <TableCell>INV001</TableCell>
        </TableRow>
    </TableBody>
</Table>;
```

## Chart

`Chart` draws a TanStack Charts definition as an SVG in the theme's `chart1`–`chart5` series colors with the foreground for axes and grid, moves the focus between points with the arrow keys and announces each in a status, and reads its source rows out as a visually hidden `table`.

```tsx
import { barY, Chart, defineChart } from "@destack/ui/chart";
import { pie, polar, radialArc } from "@destack/ui/chart/polar";
import { scaleBand } from "@destack/ui/chart/scales/band";
import { scaleLinear } from "@destack/ui/chart/scales/linear";
import { tooltip } from "@destack/ui/chart/tooltip";

<Chart
    label="Notes per notebook"
    aspectRatio={16 / 9}
    definition={defineChart({
        marks: [barY(notebooks, { x: "notebook", y: "notes" })], // lineY, areaY, barX, dot alike
        scales: { x: { scale: () => scaleBand() }, y: { scale: scaleLinear, grid: true } },
        tooltip,
    })}
    table={{
        rows: notebooks,
        columns: [
            { key: "notebook", label: "Notebook" },
            { key: "notes", label: "Notes" },
        ],
    }}
    onSelect={(point) => open(point?.datum)}
/>;

// a donut
defineChart({
    marks: [
        polar({ marks: [radialArc(pie(notebooks, { value: "notes" }), { color: "notebook" })] }),
    ],
    scales: { x: null, y: null },
});
```

### Renderers

`renderer` draws the scene as SVG by default, readable before the script runs, or on a canvas for large datasets.

```tsx
<Chart label="Ten thousand response times" renderer="canvas" definition={definition} />
```

### Entry points

`@destack/ui/chart` carries the whole TanStack Charts API beside `Chart`, and `@destack/ui/chart/<entry>` each of its entry points, which `bun run generate` writes from the pinned release.

```ts
import { binX } from "@destack/ui/chart/transform/bin";
import { zoomX } from "@destack/ui/chart/interaction/zoom";
import { treemap } from "@destack/ui/chart/hierarchy/treemap";
import { sankeyDiagram } from "@destack/ui/chart/network/sankey";
```

## Data table

`createTable` runs a headless table over reactive options, manual or grouped or nested, and `DataTable` renders it as a grid that the arrow keys, Home, End, Page Up and Page Down move through, beside `DataTableFilter`, `DataTableViewOptions` and `DataTablePagination`.

```tsx
import { createColumnHelper, rowSortingFeature, createSortedRowModel, tableFeatures } from "@tanstack/table-core";
import { createTable, DataTable, DataTableColumnHeader, DataTablePagination, selectionColumn } from "@destack/ui/data-table";

const features = tableFeatures({ rowSortingFeature, sortedRowModel: createSortedRowModel(), ... });
const column = createColumnHelper<typeof features, Note>();
const columns = column.columns([
    selectionColumn<typeof features, Note>(),
    column.accessor("title", {
        header: (context) => <DataTableColumnHeader column={context.column} title="Title" />,
    }),
]);

const table = createTable({ features, columns, get data() { return notes(); }, getRowId: (note) => note.id });
<DataTable table={table} rowHeight={40} />;
<DataTablePagination table={table} />;

createTable({ ..., manualSorting: true, manualPagination: true, rowCount: count(), state: { sorting: sorting() }, onSortingChange: setSorting });

createTable({ features: tableFeatures({ columnGroupingFeature, groupedRowModel: createGroupedRowModel(), rowExpandingFeature, expandedRowModel: createExpandedRowModel(), ... }), initialState: { grouping: ["notebook"] }, ... });
createTable({ ..., columns: [expansionColumn(), ...], getSubRows: (task) => task.subtasks });
```

## Tree

`Tree` shows nested items after the APG tree view pattern.

```tsx
import { Tree, TreeItem } from "@destack/ui/tree";

<Tree aria-label="Notebooks" onValueChange={open}>
    <TreeItem value="trips" label="Trips" defaultExpanded>
        <TreeItem value="lisbon" label="Lisbon" />
    </TreeItem>
    <TreeItem value="work" label="Work" expanded={isOpen()} onExpandedChange={setOpen}>
        <TreeItem value="plans" label="Plans" />
    </TreeItem>
</Tree>;
```

## Calendar

`Calendar` selects a day, several days or a range of `PlainDate`s, in the locale's names, numerals and first weekday.

```tsx
import { Calendar, Day } from "@destack/ui/calendar";

<Calendar
    value={day()}
    onValueChange={setDay}
    isDisabled={(day) => PlainDate.compare(day, Day.today()) < 0}
/>;
<Calendar mode="range" value={trip()} onValueChange={setTrip} />; // days marked data-range="start", "middle" or "end"
<Calendar mode="multiple" value={days()} onValueChange={setDays} />;
// de-AT: "Oktober 2026", Mo Di Mi Do Fr Sa So; en-US: "October 2026", Sun Mon …
<Calendar mode="range" months={2} min={Day.today()} />;
<Calendar captionLayout="dropdown" weekNumbers outsideDays={false} max={Day.today()} />; // ISO 8601 week numbers
```

## Date picker

`DatePicker` types a day into a `DateField`, or a range into two, beside a button that opens a `Calendar` in a popover on its focused day, typing and picking keeping one value.

```tsx
import { DatePicker } from "@destack/ui/date-picker";

<DatePicker aria-label="Due date" value={due()} onValueChange={setDue} />; // reports { year: 2026, month: 10, day: 7 }
<DatePicker aria-label="Due date" open={isOpen()} onOpenChange={setOpen} />;
<DatePicker mode="range" aria-label="Trip" onValueChange={setTrip} />; // fields named Start date and End date
<DatePicker
    mode="range"
    aria-label="Edited"
    presets={[{ label: "Last 7 days", value: lastWeek }]}
/>;
<DatePicker mode="multiple" aria-label="Days off" />; // a button named "Days off 2 dates"

<DatePicker aria-label="Remind on" value={day()} onValueChange={setDay} />;
<TimeField aria-label="Remind at" value={time()} onValueChange={setTime} />;
```

## Date field

`DateField` types a `PlainDate` as one spinbutton per year, month and day in the locale's order, which digits fill, the up and down arrow keys step and the side arrow keys move between.

```tsx
import { DateField } from "@destack/ui/date-field";

<Field>
    <FieldLabel>Birthday</FieldLabel>
    <DateField value={birthday()} onValueChange={setBirthday} />
</Field>;
// en-US: [mm]/[dd]/[yyyy], de-AT: [dd].[mm].[yyyy]
```

## Time field

`TimeField` types a `PlainTime` as one spinbutton per hour and minute, with a day period on a 12-hour clock, after the locale's hour cycle.

```tsx
import { TimeField } from "@destack/ui/time-field";

<TimeField aria-label="Reminder" value={time()} onValueChange={setTime} />;
// en-US: [09]:[30] [PM] reports "21:30", de-AT: [21]:[30]
```

## Styles

Every component passes its other attributes to its native element, composes the theme's text styles, applies the StyleX styles given as `xstyle` after its own and the inline `style` after those, marks each part with `data-slot` and its state with `data-state`, and takes its state controlled or uncontrolled.

```tsx
const styles = style.create({ wide: { width: "100%" } });
<Button xstyle={styles.wide} onClick={save}>
    Save
</Button>;
<Toggle pressed={isBold()} onPressedChange={setBold} aria-label="Bold" />;
<Toggle defaultPressed aria-label="Italic" />;
```

## Built-in text

Text the components write themselves renders in the reader's locale through `useLocale`, which also sets the arrow keys' direction.

```tsx
const locale = useLocale();
<Button aria-label={locale.render(t`Close`)} />;
// "Breadcrumb", "Pagination", "Previous", "Next", "More", "Loading", "Command palette", ...
```

## Examples and scenarios

Each component declares an example per visible state with `defineExample`, and a scenario per keyboard or focus behaviour it implements with `defineScenario`, which the tests play through the DOM.

```tsx
export const fieldNotebookForm = defineExample({
    of: Field,
    name: "notebook-form",
    description: "a notebook form whose name field reports an error until it has a value",
    render: () => <form>...</form>,
});

export const tabsSelectWithArrowKeys = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "select-with-arrow-keys",
    description:
        "select tabs as the arrow keys move the focus, skipping disabled tabs and wrapping",
    given: { examples: [tabsHistoryUnavailable] },
    when: [
        { action: "focus", target: { role: "tab", name: "Edit" } },
        { action: "press", key: "ArrowRight" },
    ],
    then: {
        observe: { selected: { kind: "name", target: { role: "tab", selected: true } } },
        each: [{ selected: "Edit" }, { selected: "Preview" }],
    },
});
```

## Credits

The components follow the APIs and styles of [shadcn/ui](https://github.com/shadcn-ui/ui), MIT licensed, copyright (c) 2023 shadcn, and their collections, focus, selection and positioning follow the architecture and terms of [React Aria](https://github.com/adobe/react-spectrum), Apache 2.0 licensed, copyright 2020 Adobe.
