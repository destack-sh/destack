import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Button } from "./button.tsx";

/** A note's toolbar: a primary save, a quiet cancel and an icon-only delete. */
export const buttonNoteToolbar = defineExample({
    of: Button,
    name: "note-toolbar",
    description: "a note's toolbar: a primary save, a quiet cancel and an icon-only delete",
    render: () => (
        <div role="toolbar" aria-label="Note">
            <Button type="submit">Save</Button>
            <Button variant="ghost">Cancel</Button>
            <Button variant="destructive" size="icon" aria-label="Delete note">
                <Icon name="trash" />
            </Button>
        </div>
    ),
});

/** A note's toolbar with every button unavailable. */
export const buttonNoteToolbarDisabled = defineExample({
    of: Button,
    name: "note-toolbar-disabled",
    description: "a note's toolbar with every button unavailable",
    render: () => (
        <div role="toolbar" aria-label="Note">
            <Button type="submit" disabled>
                Save
            </Button>
            <Button variant="ghost" disabled>
                Cancel
            </Button>
            <Button variant="destructive" size="icon" aria-label="Delete note" disabled>
                <Icon name="trash" />
            </Button>
        </div>
    ),
});
