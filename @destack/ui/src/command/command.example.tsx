import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import { toast } from "../toast/index.ts";
import {
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
    CommandShortcut,
} from "./command.tsx";

/** Show a command palette that opens with ⌘K and runs note actions. */
export function CommandExample(): JSX.Element {
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
                        <CommandItem keywords={["trash"]} onSelect={() => toast("Note deleted")}>
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
}
