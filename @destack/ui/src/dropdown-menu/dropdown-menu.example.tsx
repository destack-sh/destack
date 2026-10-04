import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuShortcut,
    DropdownMenuSub,
    DropdownMenuSubContent,
    DropdownMenuSubTrigger,
    DropdownMenuTrigger,
} from "./dropdown-menu.tsx";

/** Show a note's actions behind a button. */
export function DropdownMenuExample(): JSX.Element {
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
}
