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
