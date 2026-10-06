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
