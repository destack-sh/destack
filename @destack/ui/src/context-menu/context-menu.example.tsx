import type { JSX } from "@solidjs/web";
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuShortcut,
    ContextMenuTrigger,
} from "./context-menu.tsx";

/** Show the actions of a note card on right click. */
export function ContextMenuExample(): JSX.Element {
    return (
        <ContextMenu>
            <ContextMenuTrigger>Groceries: milk, bread, apples</ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem>
                    Open
                    <ContextMenuShortcut>↵</ContextMenuShortcut>
                </ContextMenuItem>
                <ContextMenuItem>Duplicate</ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem variant="destructive">Delete</ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    );
}
