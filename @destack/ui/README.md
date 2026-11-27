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
- `ActionButton`, `ActionSelect`, `ActionMenu`, `ActionBar` and `ActionPalette`: the page's actions, from `@destack/ui/action`.
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

| Group                  | Entries                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Forms                  | `autocomplete`, `button`, `button-group`, `calendar`, `checkbox`, `color-picker`, `color-swatch-picker`, `combobox`, `copy-button`, `date-field`, `date-picker`, `date-time-field`, `dimension-field`, `cropper`, `dropzone`, `editable`, `emoji-picker`, `field`, `form`, `icon-picker`, `input`, `input-group`, `label`, `number-field`, `otp-field`, `paint-picker`, `password-field`, `radio-group`, `search-field`, `select`, `slider`, `switch`, `tag-field`, `textarea`, `time-field`, `toggle`, `toggle-group`, `value-field` |
| Overlays               | `alert-dialog`, `context-menu`, `dialog`, `drawer`, `dropdown-menu`, `floating-panel`, `hover-card`, `menubar`, `popover`, `sheet`, `toast`, `tooltip`                                                                                                                                                                                                                                                                                                                                                                                |
| Navigation             | `breadcrumb`, `command`, `navigation-menu`, `overflow`, `pagination`, `palette`, `sidebar`, `tabs`, `toolbar`                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Data                   | `avatar`, `badge`, `card`, `carousel`, `chart`, `code-block`, `data-list`, `data-table`, `diff`, `feed`, `file-item`, `filter`, `gallery`, `grid-list`, `highlight`, `item`, `kbd`, `list-box`, `load-more`, `object`, `player`, `sort`, `table`, `tree`, `viewer`, `virtual-list`                                                                                                                                                                                                                                                    |
| Conversations          | `activity`, `bubble`, `composer`, `markdown`, `marker`, `message`, `message-scroller`, `quote`, `reaction`, `rich-text`                                                                                                                                                                                                                                                                                                                                                                                                               |
| Views                  | `board`, `event-calendar`, `timeline`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Feedback and structure | `accordion`, `alert`, `collapsible`, `empty`, `meter`, `pane`, `progress`, `prose`, `relative-time`, `resizable`, `scroll-area`, `separator`, `skeleton`, `spinner`, `visually-hidden`                                                                                                                                                                                                                                                                                                                                                |
| Layout                 | `stack`, `cluster`, `grid`, `center`, `switcher`, `cover`, `content`                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Building blocks        | `action`, `part`, `collection`, `focus`, `layer`, `position`, `selection`, `interaction`, `motion`, `drag`, `sortable`, `toggle-state`, `virtualizer`, `glyph`                                                                                                                                                                                                                                                                                                                                                                        |

### Parts

A part renders one element with `data-slot` and its state as `data-*` attributes.
`xstyle` adds styles after its own, and `render` swaps its element, as in Base UI.

`Prose`, `Stack` and `Switcher` style their descendants, through `@destack/ui/prose.css`, `stack.css` and `switcher.css`.

### Actions

A page renders these instead of listing its actions by hand.
Collections give their rows to actions by `reference(key)`.

| Part                             | What it shows                                                    |
| -------------------------------- | ---------------------------------------------------------------- |
| `ActionButton`, `ActionSelect`   | one action, or the value it sets                                 |
| `ActionMenu`                     | the actions on what a right click, long press or Shift+F10 is on |
| `ActionBar`                      | the actions on the selected rows                                 |
| `ActionPalette`                  | the actions where the person is, and the objects they search     |
| `ActionDialog`                   | what an action asks for                                          |
| `ShortcutDialog`, `ShortcutHint` | the keys there, and what can follow a typed one                  |

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

### Compose a chat

Put messages in a `MessageScroller`, and write the next one in a `Composer`.

```tsx
import { Bubble, BubbleContent } from "@destack/ui/bubble";
import {
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerInput,
    ComposerSubmit,
} from "@destack/ui/composer";
import { Message, MessageContent } from "@destack/ui/message";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "@destack/ui/message-scroller";
import { For } from "@destack/view";

/** A message of a chat. */
interface Line {
    /** The message's id. */
    readonly id: string;
    /** The message's text. */
    readonly text: string;
    /** Whether the reader wrote it. */
    readonly isOwn: boolean;
}

/** Show a chat that brings each question near the top and follows its answer. */
export function Chat(properties: {
    readonly lines: readonly Line[];
    readonly isAnswering: boolean;
    readonly onSend: (form: HTMLFormElement) => void;
    readonly onStop: () => void;
}) {
    return (
        <>
            <MessageScroller autoScroll defaultScrollPosition="last-anchor">
                <MessageScrollerViewport>
                    <MessageScrollerContent aria-busy={properties.isAnswering ? "true" : undefined}>
                        <For each={properties.lines} keyed={(line) => line.id}>
                            {(line) => (
                                <MessageScrollerItem
                                    messageId={line().id}
                                    scrollAnchor={line().isOwn}
                                >
                                    <Message align={line().isOwn ? "end" : "start"}>
                                        <MessageContent>
                                            <Bubble
                                                variant={line().isOwn ? "soft" : "ghost"}
                                                color={line().isOwn ? "gray" : undefined}
                                            >
                                                <BubbleContent>{line().text}</BubbleContent>
                                            </Bubble>
                                        </MessageContent>
                                    </Message>
                                </MessageScrollerItem>
                            )}
                        </For>
                    </MessageScrollerContent>
                </MessageScrollerViewport>
                <MessageScrollerButton />
            </MessageScroller>
            <Composer
                busy={properties.isAnswering}
                onStop={properties.onStop}
                onSubmit={(event) => {
                    if (event.currentTarget instanceof HTMLFormElement) {
                        properties.onSend(event.currentTarget);
                    }
                }}
            >
                <ComposerFrame>
                    <ComposerInput name="text" placeholder="Ask anything" />
                    <ComposerFooter>
                        <ComposerSubmit />
                    </ComposerFooter>
                </ComposerFrame>
            </Composer>
        </>
    );
}
```

A conversation of many thousands puts one `Virtualizer` over `useMessageScroller().viewport`, as the `virtual` example does.

### Hide what does not fit

An `Overflow` hides what does not fit and lists it in its menu, whose place decides what hides first.

```tsx
import { Overflow, OverflowMenu } from "@destack/ui/overflow";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@destack/ui/tabs";

/** Draw a project's views, the ones that do not fit in a menu. */
export function Views() {
    return (
        <Tabs defaultValue="overview">
            <Overflow>
                <TabsList aria-label="Views" variant="line">
                    <TabsTrigger value="overview">Overview</TabsTrigger>
                    <TabsTrigger value="roadmap">Roadmap</TabsTrigger>
                    <TabsTrigger value="settings">Settings</TabsTrigger>
                </TabsList>
                <OverflowMenu />
            </Overflow>
            <TabsContent value="overview">What changed</TabsContent>
            <TabsContent value="roadmap">What comes next</TabsContent>
            <TabsContent value="settings">How it is set up</TabsContent>
        </Tabs>
    );
}
```

### Arrange panes

A `PaneGroup` draws a `PaneLayout`, which the person splits, merges and resizes, and keeps contents alive as they move.
Panes also float, open in windows of their own by `href`, and take files from the desktop by `accepts`.

```tsx
import { PaneGroup, PaneLayout } from "@destack/ui/pane";
import { createSignal } from "@destack/view";

/** Draw files side by side. */
export function Files() {
    const [layout, setLayout] = createSignal(PaneLayout.of(["plan.md", "notes.md"], ["todo.md"]));
    const open = (path: string) => setLayout((all) => PaneLayout.open(all, path));

    return (
        <>
            <button type="button" onClick={() => open("ideas.md")}>
                Open
            </button>
            <PaneGroup
                layout={layout()}
                onLayoutChange={setLayout}
                content={(item) => <textarea aria-label={item().value} />}
            />
        </>
    );
}
```

### Compose a timeline

A `Timeline` draws its items without children, and each part's children replace its standard ones.

```tsx
import {
    Timeline,
    TimelineBar,
    TimelineBarCard,
    TimelineBarList,
    TimelineBody,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    TimelineMarker,
    TimelineRow,
    TimelineRowGuide,
    TimelineRowLabel,
    TimelineRowList,
    TimelineRowToggle,
    TimelineTrack,
} from "@destack/ui/timeline";

/** Draw a request's spans. */
export function Request(properties: { readonly spans: readonly TimelineItem[] }) {
    return <Timeline items={properties.spans} axis="offset" />;
}

/** Draw a request's spans as lines, with a card of one's own and a place to seek. */
export function Trace(properties: {
    /** The spans. */
    readonly spans: readonly TimelineItem[];
    /** The place sought. */
    readonly at: number;
    /** Handle the person seeking another place. */
    readonly onSeek: (at: number) => void;
}) {
    return (
        <Timeline items={properties.spans} axis="offset">
            <TimelineHeader corner="Span" />
            <TimelineBody>
                <TimelineGrid />
                <TimelineRowList>
                    {(node) => (
                        <TimelineRow>
                            <TimelineRowLabel>
                                <TimelineRowGuide />
                                <TimelineRowToggle />
                                {node().item.label}
                            </TimelineRowLabel>
                            <TimelineTrack>
                                <TimelineBarList>
                                    {(span) => (
                                        <TimelineBar variant="line">
                                            <TimelineBarCard>{span().label}</TimelineBarCard>
                                        </TimelineBar>
                                    )}
                                </TimelineBarList>
                            </TimelineTrack>
                        </TimelineRow>
                    )}
                </TimelineRowList>
                <TimelineMarker
                    aria-label="Playhead"
                    at={properties.at}
                    onAtChange={properties.onSeek}
                />
            </TimelineBody>
        </Timeline>
    );
}
```

### Count an axis in frames

An `axis` takes a scale of one's own, which says where its marks stand, what they say and where things land.

```tsx
import type { TimelineScale } from "@destack/ui/timeline";

/** The frames of a second. */
const RATE = 25;

/** The frames of a film, marked each second. */
export const frames: TimelineScale = {
    ticks: ({ range }) => {
        const first = Math.ceil(range.start / RATE);
        const count = Math.floor(range.end / RATE) - first + 1;

        return Array.from({ length: count }, (_, index) => ({ at: (first + index) * RATE }));
    },
    label: (frame) => `${String(Math.floor(frame / RATE))}s`,
    snap: Math.round,
};
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

### Play a film and a sound

A player is built from parts, which an app arranges, drops or replaces.

```tsx
import {
    AudioPlayer,
    Player,
    PlayerAudio,
    PlayerBar,
    PlayerControl,
    PlayerCaptions,
    PlayerPlayButton,
    PlayerSeekBar,
    PlayerTitle,
    usePlayer,
    VideoPlayer,
} from "@destack/ui/player";

/** Show a film with fewer parts over it, which a button outside moves. */
export function Film(properties: { readonly source: string; readonly captions: string }) {
    const control = new PlayerControl();

    return (
        <>
            <VideoPlayer
                source={properties.source}
                title="The pass"
                captions={[{ source: properties.captions, language: "en", label: "English" }]}
                control={control}
            >
                <PlayerCaptions />
                <PlayerBar>
                    <PlayerSeekBar />
                    <PlayerPlayButton />
                </PlayerBar>
            </VideoPlayer>
            <button type="button" onClick={() => control.seek(80)}>
                Go to 1:20
            </button>
        </>
    );
}

/** Tell how much of a sound is left, as a part of its own. */
function Left() {
    const player = usePlayer();
    const left = () => Math.ceil((player.length() ?? 0) - player.time());

    return <small>{left()} s left</small>;
}

/** Show a song with the usual parts, then with a part of its own. */
export function Song(properties: { readonly source: string; readonly onNext: () => void }) {
    return (
        <>
            <AudioPlayer
                source={properties.source}
                title="First Tram"
                artist="The Hill"
                onNext={properties.onNext}
                onEnded={properties.onNext}
            />
            <Player source={properties.source} title="First Tram">
                <PlayerAudio />
                <PlayerTitle />
                <PlayerSeekBar />
                <PlayerPlayButton />
                <Left />
            </Player>
        </>
    );
}
```

### List a page's actions

`Actions` at the root of a page lists what runs where the person is.

```tsx
import { ActionDialog, ActionMenu, ActionPalette, ShortcutDialog } from "@destack/ui/action";

/** Render what lists the page's actions. */
export function Actions() {
    return (
        <>
            <ActionMenu />
            <ActionDialog />
            <ActionPalette id="tasks" title="Tasks" />
            <ShortcutDialog />
        </>
    );
}
```

### Search a page's objects

`ActionPalette` finds the objects of some types on the device at once, and at the store as a person pauses.

```tsx
import type { ObjectType } from "@destack/object";
import type { ObjectReference } from "@destack/sync";
import { ActionPalette } from "@destack/ui/action";

/** Render the palette of a page's actions and its objects. */
export function Palette(properties: {
    /** The types of the objects. */
    readonly types: readonly ObjectType[];
    /** Open an object. */
    readonly onOpen: (reference: ObjectReference) => void;
}) {
    return (
        <ActionPalette
            id="tasks"
            title="Tasks"
            objects={{ types: properties.types, onOpen: properties.onOpen }}
        />
    );
}
```

### Draw rich text and Markdown

`RichText` draws the blocks of a field or of Markdown, and brings up text that is written line by line.

```tsx
import { useMarkdown } from "@destack/ui/markdown";
import { RichText } from "@destack/ui/rich-text";

/** Render an answer as an agent writes it. */
export function Answer(properties: {
    /** The Markdown so far. */
    readonly text: string;
    /** Whether it is still being written. */
    readonly isWriting: boolean;
}) {
    const blocks = useMarkdown(() => properties.text, { isStreaming: () => properties.isWriting });

    return <RichText blocks={blocks()} isStreaming={properties.isWriting} />;
}
```

### Mark what a search matches

`Highlight` marks a search in a text, the search of the list around it by default.

```tsx
import { Search } from "@destack/db";
import { Highlight } from "@destack/ui/highlight";

/** Render a note's title with a search marked. */
export function Title(properties: {
    /** The title. */
    readonly title: string;
    /** What a person typed. */
    readonly text: string;
}) {
    return <Highlight text={properties.title} search={Search.of(properties.text)} />;
}
```

### Edit values with a form

`Form` shows a draft from `useForm`, and each `Field` edits one of its values.
Problems show as a person leaves a field and go as they fix it, and a submit focuses the first.

```tsx
import { schema } from "@destack/schema";
import { Field, FieldConflict, FieldError, FieldLabel } from "@destack/ui/field";
import { Form, FormError, FormGuard, FormStatus, FormSubmit } from "@destack/ui/form";
import { Input } from "@destack/ui/input";
import { NumberField } from "@destack/ui/number-field";
import { useForm } from "@destack/view";
import { useBlocker } from "@destack/view/router";

/** A trip. */
const Trip = schema.object({
    title: schema.string().min(1).max(40),
    guests: schema.int().min(1).max(8),
});

/** A trip. */
type Trip = schema.Infer<typeof Trip>;

/** Edit a trip, keeping the edits past a reload and asking before they are left. */
export function EditTrip(properties: {
    readonly trip: Trip;
    readonly save: (trip: Trip) => Promise<void>;
}) {
    const form = useForm({
        values: () => properties.trip,
        schema: Trip,
        submit: (trip) => properties.save(trip),
        draft: {
            name: "trip",
            schema: schema.object({ title: schema.string().exactOptional() }),
        },
    });

    return (
        <Form form={form}>
            <Field field={form.field("title")}>
                <FieldLabel>Title</FieldLabel>
                <Input />
                <FieldConflict />
                <FieldError />
            </Field>
            <Field field={form.field("guests")}>
                <FieldLabel>Guests</FieldLabel>
                <NumberField min={1} max={8} />
                <FieldError />
            </Field>
            <FormError />
            <FormSubmit>Save trip</FormSubmit>
            <FormStatus />
            <FormGuard blocker={useBlocker(() => form.isEdited())} />
        </Form>
    );
}
```

### Take a number and a value

`NumberField` reads and writes a number as people of the language do.
`ValueField` takes whatever an action asks for, and an app replaces its input of any kind.
JSON of a named schema, such as a `Paint` or a `Shadow` of `@destack/style`, takes that type's own input, which an app replaces by its id, such as `style.paint`, with `named`.

```tsx
import { InputGroup, InputGroupAddon } from "@destack/ui/input-group";
import {
    NumberField,
    NumberFieldDecrement,
    NumberFieldIncrement,
    NumberFieldInput,
} from "@destack/ui/number-field";
import { ValueField, ValueFieldProvider, type ValueFields } from "@destack/ui/value-field";
import { createSignal } from "@destack/view";

/** Set a zoom between its buttons. */
export function Zoom() {
    const [zoom, setZoom] = createSignal<number | undefined>(1);

    return (
        <NumberField
            value={zoom()}
            onValueChange={setZoom}
            min={0.1}
            max={4}
            step={0.1}
            format={{ style: "percent" }}
        >
            <InputGroup>
                <InputGroupAddon>
                    <NumberFieldDecrement>−</NumberFieldDecrement>
                </InputGroupAddon>
                <NumberFieldInput aria-label="Zoom" />
                <InputGroupAddon align="inline-end">
                    <NumberFieldIncrement>+</NumberFieldIncrement>
                </InputGroupAddon>
            </InputGroup>
        </NumberField>
    );
}

/** Take a number as a range. */
const range: ValueFields["number"] = (properties) => (
    <input
        type="range"
        min={properties.type.minimum}
        max={properties.type.maximum}
        onInput={({ currentTarget }) => properties.onValueChange(Number(currentTarget.value))}
    />
);

/** Take a rating by an app's own input. */
export function Rating(properties: {
    readonly value: number;
    readonly onGive: (value: unknown) => void;
}) {
    return (
        <ValueFieldProvider inputs={{ number: range }}>
            <ValueField
                type={{ kind: "number", minimum: 0, maximum: 5, step: 1 }}
                value={properties.value}
                onValueChange={properties.onGive}
            />
        </ValueFieldProvider>
    );
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

### Conversations

Chats follow shadcn/ui, and a list of changes follows Primer.

| Prior art                     | `@destack/ui`     | Difference                                                           |
| ----------------------------- | ----------------- | -------------------------------------------------------------------- |
| shadcn/ui's `MessageScroller` | `MessageScroller` | Works without a provider. Counts messages unseen and steps by keys.  |
| shadcn/ui's `Message`         | `Message`         | Adds actions by hover, focus and long press, and a swipe.            |
| shadcn/ui's `Bubble`          | `Bubble`          | The same.                                                            |
| shadcn/ui's `Marker`          | `Marker`          | Adds `status` and `MarkerDescription`, and so serves as `Trace` too. |
| shadcn/ui's `Attachment`      | `FileItem`        | Shows uploads and opens files.                                       |
| Primer's `Timeline`           | `Activity`        | Condenses an item's badge and body with it.                          |

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
