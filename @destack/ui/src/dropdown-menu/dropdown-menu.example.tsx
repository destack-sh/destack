import { t } from "@destack/locale";
import { defineExample, type PropertiesOf } from "@destack/package/declare";
import { Button } from "../button/index.ts";
import { createSignal, type JSX, omit } from "@destack/view";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuShortcut,
    DropdownMenuSub,
    DropdownMenuSubContent,
    DropdownMenuSubTrigger,
    DropdownMenuTrigger,
} from "./dropdown-menu.tsx";

/** A note's actions behind a button. */
export const dropdownMenuDefault = defineExample({
    of: DropdownMenu,
    name: "default",
    description: t`A note's actions behind a button, reporting the action it runs.`,
    render: (properties) => <NoteMenu {...properties} />,
});

/** A note's actions open below their button. */
export const dropdownMenuOpen = defineExample({
    of: DropdownMenu,
    name: "open",
    description: t`A note's actions open below their button.`,
    properties: { defaultOpen: true },
    render: (properties) => <NoteMenu {...properties} />,
});

/** A note's actions, duplicating it unavailable. */
export const dropdownMenuItemDisabled = defineExample({
    of: DropdownMenuItem,
    name: "disabled",
    description: t`A note's actions, duplicating it unavailable.`,
    properties: { disabled: true },
    render: (properties) => <NoteMenu duplicate={properties} />,
});

/** A note's actions with options that check. */
export const dropdownMenuCheckable = defineExample({
    of: DropdownMenu,
    name: "checkable",
    description: t`A note's actions with a box that pins it and the order its list sorts by.`,
    render: (properties) => <NoteMenu {...properties} isCheckable />,
});

/** A note's actions open beside their button, laid out by the menu's side and alignment. */
export const dropdownMenuContentDefault = defineExample({
    of: DropdownMenuContent,
    name: "default",
    description: t`A note's actions open beside their button, laid out by the menu's side and alignment.`,
    render: (properties) => <NoteMenu defaultOpen content={properties} />,
});

/** A note's menu its owner opens and closes, reporting each request to change it. */
export const dropdownMenuControlled = defineExample({
    of: DropdownMenu,
    name: "controlled",
    description: t`A note's actions its owner opens and closes, reporting each request to change it.`,
    properties: { open: false },
    render: (properties) => {
        const [requested, setRequested] = createSignal("");

        return (
            <>
                <NoteMenu {...properties} onOpenChange={(open) => setRequested(String(open))} />
                <output aria-label="Requested">{requested()}</output>
            </>
        );
    },
});

/** A note's options, pinning it set on its own. */
export const dropdownMenuCheckboxItemDefault = defineExample({
    of: DropdownMenuCheckboxItem,
    name: "default",
    description: t`A note's options, pinning it set on its own.`,
    render: (properties) => <NoteMenu defaultOpen isCheckable pinned={properties} />,
});

/** A note's options, ordering by title set on its own. */
export const dropdownMenuRadioItemDefault = defineExample({
    of: DropdownMenuRadioItem,
    name: "default",
    description: t`A note's options, ordering by title set on its own.`,
    render: (properties) => <NoteMenu defaultOpen isCheckable title={properties} />,
});

/** Render a note's actions, reporting the action they run, varying an item or the options. */
function NoteMenu(
    properties: PropertiesOf<typeof DropdownMenu> & {
        /** The properties of the item that duplicates the note. */
        readonly duplicate?: PropertiesOf<typeof DropdownMenuItem>;
        /** Whether the menu has options that check. */
        readonly isCheckable?: boolean;
        /** The properties of the option that pins the note. */
        readonly pinned?: PropertiesOf<typeof DropdownMenuCheckboxItem>;
        /** The properties of the option that orders by title. */
        readonly title?: PropertiesOf<typeof DropdownMenuRadioItem>;
        /** The side and alignment of the open menu. */
        readonly content?: PropertiesOf<typeof DropdownMenuContent>;
    },
): JSX.Element {
    // report the action run, the pin and the order
    const [action, setAction] = createSignal("");
    const [isPinned, setPinned] = createSignal(false);
    const [order, setOrder] = createSignal("edited");

    return (
        <>
            <DropdownMenu
                {...omit(properties, "duplicate", "isCheckable", "pinned", "title", "content")}
            >
                <DropdownMenuTrigger
                    render={(attributes) => <Button {...attributes} variant="outline" />}
                >
                    Note
                </DropdownMenuTrigger>
                <DropdownMenuContent {...properties.content}>
                    <DropdownMenuLabel>Groceries</DropdownMenuLabel>
                    <DropdownMenuItem onSelect={() => setAction("rename")}>
                        Rename
                        <DropdownMenuShortcut keys="mod+r" />
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        onSelect={() => setAction("duplicate")}
                        {...properties.duplicate}
                    >
                        Duplicate
                    </DropdownMenuItem>
                    <DropdownMenuSub>
                        <DropdownMenuSubTrigger>Move to</DropdownMenuSubTrigger>
                        <DropdownMenuSubContent>
                            <DropdownMenuItem onSelect={() => setAction("trips")}>
                                Trips
                            </DropdownMenuItem>
                            <DropdownMenuItem onSelect={() => setAction("work")}>
                                Work
                            </DropdownMenuItem>
                        </DropdownMenuSubContent>
                    </DropdownMenuSub>
                    {properties.isCheckable === true ? (
                        <>
                            <DropdownMenuSeparator />
                            <DropdownMenuCheckboxItem
                                checked={isPinned()}
                                onCheckedChange={setPinned}
                                {...properties.pinned}
                            >
                                Pinned
                            </DropdownMenuCheckboxItem>
                            <DropdownMenuRadioGroup value={order()} onValueChange={setOrder}>
                                <DropdownMenuRadioItem value="edited">
                                    Last edited
                                </DropdownMenuRadioItem>
                                <DropdownMenuRadioItem value="title" {...properties.title}>
                                    Title
                                </DropdownMenuRadioItem>
                            </DropdownMenuRadioGroup>
                        </>
                    ) : undefined}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onSelect={() => setAction("delete")}>
                        Delete
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
            <output aria-label="Action">{action()}</output>
        </>
    );
}
