import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Cluster } from "../cluster/index.ts";
import { Badge } from "./badge.tsx";

/** A note's state and its number of comments. */
export const badgeNoteState = defineExample({
    of: Badge,
    name: "noteState",
    description: t`A note's state and its number of comments.`,
    properties: {
        variant: "secondary",
    },
    render: (properties) => (
        <Cluster>
            Groceries
            <Badge {...properties}>Draft</Badge>
            <Badge>3 comments</Badge>
        </Cluster>
    ),
});

/** The tags of a task, each tinted in its swatch. */
export const badgeTaskTags = defineExample({
    of: Badge,
    name: "taskTags",
    description: t`The tags of a task, each tinted in its swatch.`,
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
