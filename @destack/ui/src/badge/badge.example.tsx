import { defineExample } from "@destack/package/declare";
import { Cluster } from "../cluster/index.ts";
import { Badge } from "./badge.tsx";

/** A note's state and its number of comments. */
export const badgeNoteState = defineExample({
    of: Badge,
    name: "note-state",
    description: "a note's state and its number of comments",
    properties: {
        variant: "secondary",
    },
    render: (properties) => (
        <p>
            Groceries <Badge {...properties}>Draft</Badge> <Badge>3 comments</Badge>
        </p>
    ),
});

/** The tags of a task, each tinted in its swatch. */
export const badgeTaskTags = defineExample({
    of: Badge,
    name: "task-tags",
    description: "the tags of a task, each tinted in its swatch",
    properties: {
        color: "green",
    },
    render: (properties) => (
        <Cluster>
            <Badge {...properties}>Site</Badge>
            <Badge color="blue">Story</Badge>
            <Badge color="purple">Plan</Badge>
            <Badge color="red">At risk</Badge>
        </Cluster>
    ),
});
