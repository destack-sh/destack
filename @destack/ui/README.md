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
- `Press`, `LongPress`, `Hover`, `Move`, `Zoom` and `Swipe`: gestures, after React Aria.
- `DragManager`, `Draggable` and `Droppable`: drags by pointer, touch and keys, after dnd-kit.
- `Virtualizer`: the items of a long list in view, after TanStack Virtual, under `VirtualList` and `Feed`.
- `@destack/ui/data-table` and `@destack/ui/chart`: TanStack Table's and TanStack Charts' engines, exported with the parts.

Style a button and show a toast:

```tsx
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { useToast } from "@destack/ui/toast";

// a full-width style
const styles = style.create({ wide: { width: "100%" } });

// save, then confirm with a toast of the `Toaster` around the page
export const Save = () => {
    const toast = useToast();

    return (
        <Button variant="outline" xstyle={styles.wide} onClick={() => toast.success("Saved")}>
            Save
        </Button>
    );
};
```

## Reference

### Entries

Each entry is `@destack/ui/<entry>`.

| Group                  | Entries                                                                                                                                                                                                                                                                                                                                            |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Forms                  | `autocomplete`, `button`, `button-group`, `calendar`, `checkbox`, `color-swatch-picker`, `combobox`, `copy-button`, `date-field`, `date-picker`, `dropzone`, `emoji-picker`, `field`, `icon-picker`, `input`, `input-group`, `input-otp`, `label`, `radio-group`, `select`, `slider`, `switch`, `textarea`, `time-field`, `toggle`, `toggle-group` |
| Overlays               | `alert-dialog`, `context-menu`, `dialog`, `drawer`, `dropdown-menu`, `hover-card`, `menubar`, `popover`, `sheet`, `toast`, `tooltip`                                                                                                                                                                                                               |
| Navigation             | `breadcrumb`, `command`, `navigation-menu`, `pagination`, `palette`, `sidebar`, `tabs`                                                                                                                                                                                                                                                             |
| Data                   | `avatar`, `badge`, `card`, `carousel`, `chart`, `code-block`, `data-list`, `data-table`, `diff`, `feed`, `file-item`, `grid-list`, `item`, `kbd`, `list-box`, `load-more`, `object`, `table`, `tree`, `virtual-list`                                                                                                                               |
| Views                  | `board`, `event-calendar`, `timeline`                                                                                                                                                                                                                                                                                                              |
| Feedback and structure | `accordion`, `alert`, `aspect-ratio`, `collapsible`, `empty`, `progress`, `prose`, `resizable`, `scroll-area`, `separator`, `skeleton`, `spinner`, `visually-hidden`                                                                                                                                                                               |
| Layout                 | `stack`, `cluster`, `grid`, `center`, `switcher`, `cover`, `content`                                                                                                                                                                                                                                                                               |
| Building blocks        | `action`, `part`, `collection`, `focus`, `layer`, `position`, `selection`, `interaction`, `drag`, `sortable`, `toggle-state`, `virtualizer`, `glyph`                                                                                                                                                                                               |

### Parts

A part renders one element with `data-slot` and its state as `data-*` attributes.
`xstyle` adds styles after its own, and `render` swaps its element, as in Base UI.

`Prose`, `Stack` and `Switcher` style their descendants, through `@destack/ui/prose.css`, `stack.css` and `switcher.css`.

### Conventions

Parts, triggers, controlled state and delays follow Base UI, verbs follow React Aria, and layouts follow Every Layout.

```tsx
import { Button } from "@destack/ui/button";
import { Dialog, DialogTrigger } from "@destack/ui/dialog";
import { Tabs } from "@destack/ui/tabs";
import { TooltipProvider } from "@destack/ui/tooltip";
import { createSignal } from "@destack/view";

/** Show a dialog's trigger in tabs under a tooltip group. */
export const Note = () => {
    const [tab, setTab] = createSignal("preview");

    return (
        // Base UI's delays
        <TooltipProvider delay={500} closeDelay={100} timeout={300}>
            {/* Base UI's controlled state */}
            <Tabs value={tab()} onValueChange={setTab}>
                <Dialog>
                    {/* Base UI's render, as a function */}
                    <DialogTrigger
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        Rename
                    </DialogTrigger>
                </Dialog>
            </Tabs>
        </TooltipProvider>
    );
};
```

### State attributes

Parts mark their state with Base UI's flags, and with the same shape where Base UI has none.

```css
[data-slot="popover-content"][data-open] {
}
[data-slot="popover-trigger"][data-popup-open] {
}
[data-slot="checkbox"][data-indeterminate] {
}
/* dnd-kit has none */
[data-slot="sortable-container"][data-over] {
}
[data-slot="sortable-container"][data-refused] {
}
/* shadcn/ui's resizable has data-state */
[data-slot="resizable-panel"][data-collapsed] {
}
/* Base UI has no carousel */
[data-slot="carousel-play"][data-playing] {
}
```

## Examples

### Rename in a dialog

A dialog's parts follow shadcn/ui, with a field inside and close buttons that submit or cancel.

```tsx
import { Button } from "@destack/ui/button";
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
        <DialogTrigger render={(attributes) => <Button {...attributes} variant="outline" />}>
            Rename
        </DialogTrigger>
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
                <DialogClose render={(attributes) => <Button {...attributes} variant="outline" />}>
                    Cancel
                </DialogClose>
                <DialogClose render={(attributes) => <Button {...attributes} type="submit" />}>
                    Save
                </DialogClose>
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

### Press and move an element

Spread a gesture's attributes on its element.

```tsx
import { Move, Press } from "@destack/ui/interaction";
import { mergeProperties } from "@destack/ui/part";
import { useEnvironment } from "@destack/view";

/** Show a pin that opens on a press and follows a pointer that stays down. */
export function Pin(properties: {
    readonly onOpen: () => void;
    readonly onMove: (x: number, y: number) => void;
}) {
    const { clock } = useEnvironment();
    const press = new Press({ onPress: () => properties.onOpen() });
    const move = new Move(
        { onMove: (event) => properties.onMove(event.offsetX, event.offsetY) },
        clock,
    );

    return <button {...mergeProperties(press.attributes(), move.attributes())}>Pin</button>;
}
```

### Reorder a list

`Sortable` reports where an item was dropped.

```tsx
import { Sortable, SortableContainer, SortableHandle, SortableItem } from "@destack/ui/sortable";
import { createSignal, For } from "@destack/view";

/** Show steps in the order people leave them. */
export function Steps() {
    const [steps, setSteps] = createSignal(["Pack", "Check in", "Board"]);
    const place = (id: string, next: string | undefined) =>
        setSteps((current) => {
            const rest = current.filter((step) => step !== id);
            const at = next === undefined ? rest.length : rest.indexOf(next);

            return [...rest.slice(0, at), id, ...rest.slice(at)];
        });

    return (
        <Sortable onMove={(move) => place(move.id, move.next)}>
            <SortableContainer id="steps" label="Steps">
                <For each={steps()}>
                    {(step) => (
                        <SortableItem id={step}>
                            <SortableHandle /> {step}
                        </SortableItem>
                    )}
                </For>
            </SortableContainer>
        </Sortable>
    );
}
```

### Show a long list

Give `VirtualList` every item and a function that renders one.

```tsx
import { VirtualList } from "@destack/ui/virtual-list";

/** Show a log's lines, only the ones in view in the document. */
export function Log(properties: { readonly lines: readonly string[] }) {
    return (
        <VirtualList each={properties.lines} estimateSize={() => 32} style={{ height: "20rem" }}>
            {(line) => <div>{line}</div>}
        </VirtualList>
    );
}
```

### Follow a conversation

Anchor a `Feed` to its end and load older items from its start.

```tsx
import { Feed } from "@destack/ui/feed";

/** A message of a conversation. */
interface Message {
    /** The message's id. */
    readonly id: string;
    /** The message's text. */
    readonly text: string;
}

/** Show a conversation from its newest message, following new ones. */
export function Conversation(properties: {
    readonly messages: readonly Message[];
    readonly isLoading: boolean;
    readonly onLoadOlder: () => void;
}) {
    return (
        <Feed
            each={properties.messages}
            label={(message) => message.text}
            itemKey={(message) => message.id}
            estimateSize={() => 48}
            anchorTo="end"
            followOnAppend
            onLoadMore={properties.onLoadOlder}
            isLoading={properties.isLoading}
            aria-label="Conversation"
            style={{ height: "24rem" }}
        >
            {(message) => <p>{message.text}</p>}
        </Feed>
    );
}
```

### Compose a timeline

Arrange a `Timeline` from its parts and render each row yourself.

```tsx
import {
    Timeline,
    TimelineBar,
    TimelineBody,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    TimelineRow,
    TimelineRowLabel,
    TimelineRowList,
} from "@destack/ui/timeline";

/** Show a trace without a minimap, its failed spans solid red. */
export function Trace(properties: {
    readonly spans: readonly TimelineItem[];
    readonly failed: ReadonlySet<string>;
}) {
    return (
        <Timeline items={properties.spans} axis="offset" rowHeight={28}>
            <TimelineHeader corner="Span" />
            <TimelineBody>
                <TimelineGrid />
                <TimelineRowList>
                    {(node) => (
                        <TimelineRow>
                            <TimelineRowLabel />
                            {properties.failed.has(node().item.key) ? (
                                <TimelineBar color="red" variant="solid" />
                            ) : (
                                <TimelineBar />
                            )}
                        </TimelineRow>
                    )}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    );
}
```

### Compose a calendar

Give an `EventCalendar` the views it offers; each shows while chosen, and the views toggle lists them.

```tsx
import {
    type CalendarEvent,
    EventCalendar,
    EventCalendarAgenda,
    EventCalendarBody,
    EventCalendarHeader,
    EventCalendarMonth,
    EventCalendarTime,
    EventCalendarYear,
} from "@destack/ui/event-calendar";

/** Show a team's calendar by day, four days, week, month, year and agenda, nine to five shaded. */
export function TeamCalendar(properties: { readonly events: readonly CalendarEvent[] }) {
    const hours = [{ start: "09:00", end: "17:00" }] as const;

    return (
        <EventCalendar
            events={properties.events}
            defaultView="week"
            dayCount={4}
            weekNumbers
            workWeek={{ 1: hours, 2: hours, 3: hours, 4: hours, 5: hours }}
        >
            <EventCalendarHeader />
            <EventCalendarBody>
                <EventCalendarTime />
                <EventCalendarMonth />
                <EventCalendarYear />
                <EventCalendarAgenda />
            </EventCalendarBody>
        </EventCalendar>
    );
}
```

Arrange a view from its parts to replace any day, column or date yourself.

```tsx
import {
    type CalendarEvent,
    EventCalendar,
    EventCalendarColumn,
    EventCalendarColumnContent,
    EventCalendarDayColumnList,
    EventCalendarDayHeader,
    EventCalendarHeader,
    EventCalendarTimeAxis,
    EventCalendarHourGrid,
    EventCalendarTimeGrid,
    EventCalendarLanding,
    EventCalendarTime,
    useEventCalendarDay,
} from "@destack/ui/event-calendar";

/** Show a week without the now line, its headings showing the date alone. */
export function Schedule(properties: {
    readonly events: readonly CalendarEvent[];
    readonly onEventChange: (event: CalendarEvent) => void;
}) {
    return (
        <EventCalendar
            events={properties.events}
            defaultView="week"
            onEventChange={properties.onEventChange}
        >
            <EventCalendarHeader />
            <EventCalendarTime>
                <EventCalendarDayHeader>{() => <DateHeading />}</EventCalendarDayHeader>
                <EventCalendarTimeGrid>
                    <EventCalendarTimeAxis />
                    <EventCalendarDayColumnList>
                        {() => (
                            <EventCalendarColumn>
                                <EventCalendarHourGrid />
                                <EventCalendarColumnContent />
                                <EventCalendarLanding />
                            </EventCalendarColumn>
                        )}
                    </EventCalendarDayColumnList>
                </EventCalendarTimeGrid>
            </EventCalendarTime>
        </EventCalendar>
    );
}

/** Show a day's date alone. */
function DateHeading() {
    const { day } = useEventCalendarDay();

    return <strong>{day}</strong>;
}
```

### Import component stylesheets

A page imports each descendant stylesheet once.

```css
@import "@destack/ui/prose.css";
```

## Comparison

### shadcn/ui

Components, parts and props follow shadcn/ui and Radix, and some follow their own prior art, such as cmdk's `Command` and Sonner's `Toaster`.

| shadcn/ui               | `@destack/ui`                 | Difference                                         |
| ----------------------- | ----------------------------- | -------------------------------------------------- |
| copied React components | Solid components in a package | Styled with StyleX instead of Tailwind.            |
| Radix's Popper          | CSS anchor positioning        | Dialogs, popovers and drawers are native elements. |
| English strings         | the reader's locale           | Component text is translated.                      |

### React Aria and dnd-kit

Gestures follow React Aria and drags follow dnd-kit.

| Prior art                          | `@destack/ui` | Difference                                          |
| ---------------------------------- | ------------- | --------------------------------------------------- |
| React Aria's `usePress`            | `Press`       | Adds `keys`. Positions are in the viewport.         |
| React Aria's `useLongPress`        | `LongPress`   | A pen long-presses too.                             |
| React Aria's `useHover`            | `Hover`       | The same.                                           |
| React Aria's `useMove`             | `Move`        | Starts after a distance, or after a finger's hold.  |
| use-gesture's pinch                | `Zoom`        | Reports the factor of each step.                    |
| dnd-kit's `DndContext` and sensors | `DragManager` | A drag targets a place in a list, a cell or a time. |
| dnd-kit's `DragOverlay`            | `DragPreview` | Shows a copy of the lifted item.                    |
| TanStack Virtual's `Virtualizer`   | `Virtualizer` | The same engine, with `items()` and `totalSize()`.  |
| React Aria's load-more item        | `LoadMore`    | The same.                                           |
