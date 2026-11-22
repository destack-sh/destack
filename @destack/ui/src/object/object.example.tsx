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
                color="teal"
                href="/task/task-1"
            />{" "}
            until Friday.
        </p>
    ),
});

/** A task's chip in the violet swatch. */
export const objectChipTaskViolet = defineExample({
    of: ObjectChip,
    name: "task-violet",
    description: "a task's chip in the violet swatch",
    properties: {
        color: "violet",
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
