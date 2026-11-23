import { defineExample } from "@destack/package/declare";
import { ObjectChip } from "./object.tsx";

/** A task as a chip in a sentence, linking to its page. */
export const objectChipTask = defineExample({
    of: ObjectChip,
    name: "task",
    description: "a task inline with its icon, title and accent, linking to its page",
    render: () => (
        <p>
            Blocked on{" "}
            <ObjectChip
                title="Write the release notes"
                glyph={{ emoji: "📝" }}
                color="cyan"
                href="/task/task-1"
            />{" "}
            until Friday.
        </p>
    ),
});

/** A task's chip in the purple swatch. */
export const objectChipTaskPurple = defineExample({
    of: ObjectChip,
    name: "task-purple",
    description: "a task's chip in the purple swatch",
    properties: {
        color: "purple",
    },
    render: (properties) => (
        <p>
            Blocked on{" "}
            <ObjectChip
                {...properties}
                title="Write the release notes"
                glyph={{ emoji: "📝" }}
                href="/task/task-1"
            />{" "}
            until Friday.
        </p>
    ),
});
