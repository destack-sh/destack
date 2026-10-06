import { defineExample } from "@destack/package/declare";
import { Badge } from "./badge.tsx";

/** A note's state and its number of comments. */
export const badgeNoteState = defineExample({
    of: Badge,
    name: "note-state",
    description: "a note's state and its number of comments",
    render: () => (
        <p>
            Groceries <Badge variant="secondary">Draft</Badge> <Badge>3 comments</Badge>
        </p>
    ),
});
