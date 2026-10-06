import { defineExample } from "@destack/package/declare";
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuShortcut,
    ContextMenuTrigger,
} from "./context-menu.tsx";

/** The actions of a note card on right click. */
export const contextMenuNoteCardActions = defineExample({
    of: ContextMenu,
    name: "note-card-actions",
    description: "the actions of a note card on right click",
    render: () => (
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
    ),
});
