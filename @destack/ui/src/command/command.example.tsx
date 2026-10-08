import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { toast } from "../toast/index.ts";
import {
    Command,
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandLoading,
    CommandSeparator,
    CommandShortcut,
} from "./command.tsx";

/** A command palette that opens with ⌘K and runs note actions. */
export const commandDialogNoteActions = defineExample({
    of: CommandDialog,
    name: "note-actions",
    description: "a command palette that opens with ⌘K and runs note actions",
    render: () => {
        const [isOpen, setOpen] = createSignal(false);

        return (
            <div
                onKeyDown={(event) => {
                    // open the palette on ⌘K
                    if (event.key === "k" && event.metaKey) {
                        setOpen(true);
                    }
                }}
            >
                <CommandDialog open={isOpen()} onOpenChange={setOpen}>
                    <CommandInput placeholder="Type a command" />
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <CommandGroup heading="Notes">
                            <CommandItem onSelect={() => toast("Note created")}>
                                New note
                                <CommandShortcut>⌘N</CommandShortcut>
                            </CommandItem>
                            <CommandItem
                                keywords={["trash"]}
                                onSelect={() => toast("Note deleted")}
                            >
                                Delete note
                            </CommandItem>
                        </CommandGroup>
                        <CommandSeparator />
                        <CommandGroup heading="Settings">
                            <CommandItem onSelect={() => toast("Opening appearance")}>
                                Appearance
                            </CommandItem>
                        </CommandGroup>
                    </CommandList>
                </CommandDialog>
            </div>
        );
    },
});

/** A list of note and settings commands with an unavailable one, reporting the command it runs. */
export const commandNoteCommands = defineExample({
    of: Command,
    name: "note-commands",
    description:
        "a list of note and settings commands with an unavailable one, reporting the command it runs",
    render: () => {
        const [chosen, setChosen] = createSignal("");

        return (
            <>
                <Command aria-label="Commands">
                    <CommandInput placeholder="Type a command" />
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <CommandGroup heading="Notes">
                            <CommandItem onSelect={setChosen}>New note</CommandItem>
                            <CommandItem keywords={["trash"]} onSelect={setChosen}>
                                Delete note
                            </CommandItem>
                        </CommandGroup>
                        <CommandSeparator />
                        <CommandGroup heading="Settings">
                            <CommandItem disabled>Billing</CommandItem>
                            <CommandItem value="theme" onSelect={setChosen}>
                                Appearance
                            </CommandItem>
                        </CommandGroup>
                    </CommandList>
                </Command>
                <output>{chosen()}</output>
            </>
        );
    },
});

/** A command list whose owner holds the highlight, reporting the option the arrow keys ask for. */
export const commandControlledHighlight = defineExample({
    of: Command,
    name: "controlled-highlight",
    description:
        "a command list whose owner holds the highlight, reporting the option the arrow keys ask for",
    properties: { value: "Appearance" },
    render: (properties) => {
        const [requested, setRequested] = createSignal("");

        return (
            <>
                <Command {...properties} onValueChange={setRequested}>
                    <CommandInput aria-label="Command" />
                    <CommandList>
                        <CommandItem>New note</CommandItem>
                        <CommandItem>Appearance</CommandItem>
                    </CommandList>
                </Command>
                <output>{requested()}</output>
            </>
        );
    },
});

/** The note and settings commands whose search finds none, showing the empty note. */
export const commandNoteCommandsNoResults = defineExample({
    of: Command,
    name: "note-commands-no-results",
    description: "the note and settings commands whose search finds none, showing the empty note",
    render: () => (
        <Command aria-label="Commands" defaultSearch="export">
            <CommandInput placeholder="Type a command" />
            <CommandList>
                <CommandEmpty>No results</CommandEmpty>
                <CommandGroup heading="Notes">
                    <CommandItem>New note</CommandItem>
                    <CommandItem>Delete note</CommandItem>
                </CommandGroup>
            </CommandList>
        </Command>
    ),
});

/** A command list whose owner loads the matching commands, showing their loading. */
export const commandPeopleLoading = defineExample({
    of: Command,
    name: "people-loading",
    description: "a command list whose owner loads the matching commands, showing their loading",
    render: () => (
        <Command aria-label="People" shouldFilter={false} defaultSearch="ada">
            <CommandInput placeholder="Find a person" />
            <CommandList>
                <CommandLoading>Searching…</CommandLoading>
            </CommandList>
        </Command>
    ),
});
