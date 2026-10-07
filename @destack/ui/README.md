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
<Button loading>Save</Button>; // a spinner, aria-busy and disabled while the action runs
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

## Aspect ratio

`AspectRatio` keeps a box at a width-to-height ratio as its width changes.

```tsx
import { AspectRatio } from "@destack/ui/aspect-ratio";

<AspectRatio ratio={16 / 9}>
    <img src="/photos/alfama.jpg" alt="Alfama at dusk" />
</AspectRatio>;
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

`SchemaForm` renders a field per property of a JSON Schema, such as a method's input, starting from each property's `default`, and refuses values the schema refuses in the person's language through `useSchemaForm`.

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
            place: { type: "object", properties: { city: { type: "string" } } }, // a fieldset, added and removed while optional
            repeat: { oneOf: [daily, weekly] }, // a choice of kind, then its fields
        },
        required: ["title"],
    }}
    submit="File task"
    onSubmit={(value) => file(value)} // { title: "Milk", estimate: 1 } once every field is kept
/>;
schemaFields(input).length; // 0 for a command that takes nothing

// local references resolve against the root schema, recursive ones expanding only where a value exists
<SchemaForm
    schema={{
        $defs: {
            node: {
                type: "object",
                properties: {
                    name: { type: "string" },
                    children: { type: "array", items: { $ref: "#/$defs/node" } }, // rows added one by one
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

// render a property's control itself, such as a picker for a reference, undefined keeping the form's
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

`Checkbox` renders a native checkbox with an `indeterminate` state.

```tsx
import { Checkbox } from "@destack/ui/checkbox";

<Checkbox name="terms" required defaultChecked />;
<Checkbox
    aria-label="Select all"
    checked={isAll()}
    indeterminate={isSome()}
    onCheckedChange={selectAll}
/>;
```

## Radio group

`RadioGroup` gives its native radios one name and one value.

```tsx
import { RadioGroup, RadioGroupItem } from "@destack/ui/radio-group";

<RadioGroup aria-label="Density" defaultValue="regular" orientation="horizontal" required>
    <RadioGroupItem value="compact" aria-label="Compact" />
    <RadioGroupItem value="regular" aria-label="Regular" />
</RadioGroup>;
```

## Switch

`Switch` renders a native checkbox as a switch.

```tsx
import { Switch } from "@destack/ui/switch";

<Switch aria-label="Notifications" checked={isOn()} onCheckedChange={setOn} />;
```

## Slider

`Slider` renders a native range input per thumb over a track filled between its values: one value, or two for a range, along either orientation.

```tsx
import { Slider } from "@destack/ui/slider";

<Slider min={12} max={24} defaultValue={[16]} aria-label="Font size" />;
<Slider
    max={500}
    step={10}
    value={price()}
    onValueChange={setPrice}
    marks={[0, 250, 500]}
    aria-label="Price"
/>;
// the thumbs of a range are named "Minimum" and "Maximum"
```

## Progress

`Progress` renders a native progress bar.

```tsx
import { Progress } from "@destack/ui/progress";

<Progress value={30} max={100} aria-label="Upload" />;
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

## Combobox

`Combobox` filters a list of options as the person types, after the APG combobox pattern.

```tsx
import {
    Combobox,
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
        <ComboboxItem value="Trips">Trips</ComboboxItem>
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
<Combobox shouldFilter={false} onInputValueChange={search}>
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

`Command` searches options by text and keywords, and `CommandDialog` shows it as a palette.

```tsx
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "@destack/ui/command";

<Command aria-label="Commands" value={highlighted()} onValueChange={setHighlighted}>
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
// <dialog aria-labelledby aria-describedby closedby="any">, open={open()} controls it
<Dialog modal={false}>…</Dialog>; // opens with show() beside the page, which stays usable
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

`Drawer` slides a dialog in from its direction, with a handle from the bottom, resting at its snap points.

```tsx
import { Drawer, DrawerContent, DrawerTitle, DrawerTrigger } from "@destack/ui/drawer";

<Drawer direction="bottom">
    <DrawerTrigger>Share</DrawerTrigger>
    <DrawerContent>
        <DrawerTitle>Share Groceries</DrawerTitle>
    </DrawerContent>
</Drawer>;
// a swipe toward the drawer's edge past a quarter of it, or a flick, closes it

<Drawer snapPoints={[0.5, "320px", 1]} activeSnapPoint={point()} onSnapPointChange={setPoint}>
    …
</Drawer>;
// fractions of the viewport or pixel lengths, smallest first, resting at the largest by default
// a released drag settles at the snap point nearest where its speed carries it, and closes past the smallest
// data-snap-point="2" on the content, its height following the point with the theme's motion
```

## Popover

`Popover` opens its content in the top layer through the Popover API, anchored to its trigger, or as a modal dialog.

```tsx
import { Popover, PopoverContent, PopoverTrigger } from "@destack/ui/popover";

<Popover>
    <PopoverTrigger variant="outline">Size</PopoverTrigger>
    <PopoverContent side="bottom" align="start">
        …
    </PopoverContent>
</Popover>;
// without CSS anchor positioning, placeBeside sets its place on open, scroll and resize, flipping as the browser would

<Popover modal>
    <PopoverTrigger variant="outline">Rename</PopoverTrigger>
    <PopoverContent>…</PopoverContent>
</Popover>;
// a <dialog> shown with showModal: the focus stays inside, the page behind is inert and does not scroll,
// and the trigger takes the focus back on close
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
</TooltipProvider>; // the next tooltip shows at once while the last just hid
```

## Toast

`toast` shows a message in the `Toaster`, a live region above dialogs that Alt+T focuses.

```tsx
import { Toaster, toast } from "@destack/ui/toast";

<Toaster position="bottom-right" duration={4000} />;
toast("Note archived", { action: { label: "Undo", onClick: restore } });
toast.promise(save(), { loading: "Saving", success: "Saved", error: "Saving failed" });
<Toaster position="top-center" richColors />; // each kind colors its toast; a swipe off the toaster's side dismisses
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
// also DropdownMenuCheckboxItem, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSub, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuShortcut
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

`NavigationMenu` discloses panels of links from its triggers after the APG disclosure navigation pattern.

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

<NavigationMenu aria-label="Main" delayDuration={200} skipDelayDuration={300}>
    <NavigationMenuList>
        <NavigationMenuItem>
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
// viewport={false} frames each panel below its own item instead
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
// 1 … 5 6 7 … 20, links that page in place while onPageChange handles them
```

## Sidebar

`Sidebar` collapses off canvas or to icons on wide screens and opens as a sheet on narrow ones, remembered per viewer and toggled with Ctrl or ⌘ B.

```tsx
import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupLabel,
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
    <ResizableHandle /> {/* Enter collapses and restores the panel before it */}
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
    <CarouselPlay /> {/* stops and starts the rotation */}
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

// a donut: pie slices drawn as arcs in polar coordinates
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
// every entry point but the bindings of other frameworks
```

## Data table

`createTable` runs a headless table over reactive options, and `DataTable` renders it as a grid that arrow keys move through, beside `DataTableFilter`, `DataTableViewOptions` and `DataTablePagination`; a manual table leaves sorting, filtering and paging to the query its rows come from, and grouped or nested rows expand in place.

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

// group by a column into rows that expand, a treegrid whose group cells count their members
createTable({ features: tableFeatures({ columnGroupingFeature, groupedRowModel: createGroupedRowModel(), rowExpandingFeature, expandedRowModel: createExpandedRowModel(), ... }), initialState: { grouping: ["notebook"] }, ... });
// expand rows into their sub rows from a button column
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

`DatePicker` types a day into a `DateField`, or a range into two, beside a button that opens a `Calendar` in a popover, typing and picking keeping one value.

```tsx
import { DatePicker } from "@destack/ui/date-picker";

<DatePicker aria-label="Due date" value={due()} onValueChange={setDue} />;
// a group of [10]/[07]/[2026] and a calendar button: typing or picking Oct 7 reports { year: 2026, month: 10, day: 7 },
// a cleared day undefined
<DatePicker mode="range" aria-label="Trip" onValueChange={setTrip} />; // fields named Start date and End date
<DatePicker
    mode="range"
    aria-label="Edited"
    presets={[{ label: "Last 7 days", value: lastWeek }]}
/>;
<DatePicker mode="multiple" aria-label="Days off" />; // a button named "Days off 2 dates"

// a date and a time of day side by side, without a combined value
<DatePicker aria-label="Remind on" value={day()} onValueChange={setDay} />;
<TimeField aria-label="Remind at" value={time()} onValueChange={setTime} />;
```

## Date field

`DateField` types a `PlainDate` as one spinbutton per year, month and day, in the locale's order and with the locale's literals between them.

```tsx
import { DateField } from "@destack/ui/date-field";

<Field>
    <FieldLabel>Birthday</FieldLabel>
    <DateField value={birthday()} onValueChange={setBirthday} />
</Field>;
// en-US: [mm]/[dd]/[yyyy], de-AT: [dd].[mm].[yyyy], each role="spinbutton" with aria-valuenow, min, max and text
// 1 0 → month 10 and on to the day, ArrowUp and ArrowDown step and wrap, Backspace clears,
// ArrowLeft and ArrowRight move along the writing direction
// reports once every segment is filled: 2026-02-30 and a cleared segment report undefined
```

## Time field

`TimeField` types a `PlainTime` as one spinbutton per hour and minute, with a day period on a 12-hour clock, after the locale's hour cycle.

```tsx
import { TimeField } from "@destack/ui/time-field";

<TimeField aria-label="Reminder" value={time()} onValueChange={setTime} />;
// en-US: [09]:[30] [PM], typing p chooses PM, reports "21:30"; de-AT: [21]:[30]
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
// locale/de.json ships machine-translated German drafts of each, listed under drafts
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

## License

The components port shadcn/ui under the MIT License.

```text
MIT License

Copyright (c) 2023 shadcn

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
