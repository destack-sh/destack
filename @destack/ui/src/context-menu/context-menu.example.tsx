import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import {
    ContextMenu,
    ContextMenuCheckboxItem,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuLabel,
    ContextMenuRadioGroup,
    ContextMenuRadioItem,
    ContextMenuSeparator,
    ContextMenuShortcut,
    ContextMenuTrigger,
} from "./context-menu.tsx";

/** The styles of the note card a reviewer right clicks. */
const styles = style.create({
    card: {
        paddingBlock: space[3],
        paddingInline: space[4],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.card,
        color: color.foreground,
    },
});

/** The actions of a note card on right click. */
export const contextMenuNoteCardActions = defineExample({
    of: ContextMenu,
    name: "note-card-actions",
    description: "the actions of a note card on right click",
    render: () => (
        <ContextMenu>
            <ContextMenuTrigger xstyle={[text.sm, styles.card]}>
                Groceries: milk, bread, apples
            </ContextMenuTrigger>
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

/** The view options of a notebook on right click: a pin and one sort order. */
export const contextMenuNotebookView = defineExample({
    of: ContextMenu,
    name: "notebook-view",
    description: "the view options of a notebook on right click: a pin and one sort order",
    render: () => {
        const [isPinned, setPinned] = createSignal(true);
        const [order, setOrder] = createSignal("edited");

        return (
            <ContextMenu>
                <ContextMenuTrigger xstyle={[text.sm, styles.card]}>
                    Trips: 12 notes
                </ContextMenuTrigger>
                <ContextMenuContent>
                    <ContextMenuCheckboxItem checked={isPinned()} onCheckedChange={setPinned}>
                        Pinned
                    </ContextMenuCheckboxItem>
                    <ContextMenuSeparator />
                    <ContextMenuLabel>Sort by</ContextMenuLabel>
                    <ContextMenuRadioGroup value={order()} onValueChange={setOrder}>
                        <ContextMenuRadioItem value="edited">Last edited</ContextMenuRadioItem>
                        <ContextMenuRadioItem value="title">Title</ContextMenuRadioItem>
                    </ContextMenuRadioGroup>
                </ContextMenuContent>
            </ContextMenu>
        );
    },
});
