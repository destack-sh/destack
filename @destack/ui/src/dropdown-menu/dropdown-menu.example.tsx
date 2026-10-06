import { defineExample } from "@destack/package/declare";
import { createSignal } from "solid-js";
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
export const dropdownMenuNoteActions = defineExample({
    of: DropdownMenu,
    name: "note-actions",
    description: "a note's actions behind a button",
    render: () => {
        const [isPinned, setPinned] = createSignal(false);

        return (
            <DropdownMenu>
                <DropdownMenuTrigger variant="outline">Note</DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuLabel>Groceries</DropdownMenuLabel>
                    <DropdownMenuItem>
                        Rename
                        <DropdownMenuShortcut>⌘R</DropdownMenuShortcut>
                    </DropdownMenuItem>
                    <DropdownMenuCheckboxItem checked={isPinned()} onCheckedChange={setPinned}>
                        Pinned
                    </DropdownMenuCheckboxItem>
                    <DropdownMenuSub>
                        <DropdownMenuSubTrigger>Move to</DropdownMenuSubTrigger>
                        <DropdownMenuSubContent>
                            <DropdownMenuItem>Trips</DropdownMenuItem>
                            <DropdownMenuItem>Work</DropdownMenuItem>
                        </DropdownMenuSubContent>
                    </DropdownMenuSub>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive">Delete</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );
    },
});

/** A note's menu with an unavailable item and a submenu, reporting the action it runs. */
export const dropdownMenuNoteMenu = defineExample({
    of: DropdownMenu,
    name: "note-menu",
    description:
        "a note's menu with an unavailable item and a submenu, reporting the action it runs",
    render: () => {
        const [action, setAction] = createSignal("");

        return (
            <>
                <DropdownMenu>
                    <DropdownMenuTrigger>Note</DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <DropdownMenuItem onSelect={() => setAction("rename")}>
                            Rename
                        </DropdownMenuItem>
                        <DropdownMenuItem disabled>Duplicate</DropdownMenuItem>
                        <DropdownMenuSub>
                            <DropdownMenuSubTrigger>Move to</DropdownMenuSubTrigger>
                            <DropdownMenuSubContent>
                                <DropdownMenuItem onSelect={() => setAction("trips")}>
                                    Trips
                                </DropdownMenuItem>
                                <DropdownMenuItem>Work</DropdownMenuItem>
                            </DropdownMenuSubContent>
                        </DropdownMenuSub>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive">Delete</DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
                <output>{action()}</output>
            </>
        );
    },
});

/** A note list's view options with every option unavailable, reporting any change it makes. */
export const dropdownMenuUnavailableOptions = defineExample({
    of: DropdownMenu,
    name: "unavailable-options",
    description:
        "a note list's view options with every option unavailable, reporting any change it makes",
    render: () => {
        const [changed, setChanged] = createSignal("");

        return (
            <>
                <DropdownMenu>
                    <DropdownMenuTrigger>View</DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <DropdownMenuCheckboxItem
                            checked={false}
                            disabled
                            onCheckedChange={() => setChanged("archived")}
                        >
                            Show archived
                        </DropdownMenuCheckboxItem>
                        <DropdownMenuRadioGroup value="title" onValueChange={setChanged}>
                            <DropdownMenuRadioItem value="date" disabled>
                                Date
                            </DropdownMenuRadioItem>
                        </DropdownMenuRadioGroup>
                        <DropdownMenuSub>
                            <DropdownMenuSubTrigger disabled>Move to</DropdownMenuSubTrigger>
                            <DropdownMenuSubContent>
                                <DropdownMenuItem>Trips</DropdownMenuItem>
                            </DropdownMenuSubContent>
                        </DropdownMenuSub>
                    </DropdownMenuContent>
                </DropdownMenu>
                <output>{changed()}</output>
            </>
        );
    },
});

/** A note's menu its owner opens and closes, reporting each request to change it. */
export const dropdownMenuControlled = defineExample({
    of: DropdownMenu,
    name: "controlled",
    description: "a note's menu its owner opens and closes, reporting each request to change it",
    properties: { open: false },
    render: (properties) => {
        const [requested, setRequested] = createSignal("");

        return (
            <>
                <DropdownMenu {...properties} onOpenChange={(open) => setRequested(String(open))}>
                    <DropdownMenuTrigger>Note</DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <DropdownMenuItem>Rename</DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
                <output>{requested()}</output>
            </>
        );
    },
});

/** A note's actions open below their button. */
export const dropdownMenuNoteActionsOpen = defineExample({
    of: DropdownMenu,
    name: "note-actions-open",
    description: "a note's actions open below their button",
    render: () => {
        const [isPinned, setPinned] = createSignal(false);

        return (
            <DropdownMenu defaultOpen>
                <DropdownMenuTrigger variant="outline">Note</DropdownMenuTrigger>
                <DropdownMenuContent>
                    <DropdownMenuLabel>Groceries</DropdownMenuLabel>
                    <DropdownMenuItem>
                        Rename
                        <DropdownMenuShortcut>⌘R</DropdownMenuShortcut>
                    </DropdownMenuItem>
                    <DropdownMenuCheckboxItem checked={isPinned()} onCheckedChange={setPinned}>
                        Pinned
                    </DropdownMenuCheckboxItem>
                    <DropdownMenuSub>
                        <DropdownMenuSubTrigger>Move to</DropdownMenuSubTrigger>
                        <DropdownMenuSubContent>
                            <DropdownMenuItem>Trips</DropdownMenuItem>
                            <DropdownMenuItem>Work</DropdownMenuItem>
                        </DropdownMenuSubContent>
                    </DropdownMenuSub>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive">Delete</DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        );
    },
});
