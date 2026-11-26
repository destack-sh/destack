import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { textInteraction } from "../scenario/text.ts";
import { TextRope } from "./rope.ts";

/** Append, prepend, insert and delete characters a change at a time. */
export const editCharacters = defineScenario({
    of: TextRope,
    interaction: textInteraction,
    name: "editCharacters",
    description: t`Append, prepend, insert at random places and delete characters one change at a time.`,
    given: { examples: [] },
    parameters: { times: 2000 },
    when: [
        { operation: "text.append", times: "{times}", step: { action: "append", characters: 1 } },
        { operation: "text.prepend", times: "{times}", step: { action: "prepend", characters: 1 } },
        { operation: "text.type", times: "{times}", step: { action: "type", words: 1 } },
        { operation: "text.delete", times: "{times}", step: { action: "delete", characters: 3 } },
    ],
    then: [],
});

/** Edit a formatted text of about fifty thousand characters. */
export const editLongText = defineScenario({
    of: TextRope,
    interaction: textInteraction,
    name: "editLongText",
    description: t`Build a text of about fifty thousand characters with marks and paragraphs, then type, delete, mark, unmark, insert and change nodes, and read spans in it.`,
    given: { examples: [] },
    parameters: { times: 200 },
    when: [
        { times: 1000, step: { action: "type", words: 9 } },
        { times: 600, step: { action: "mark", type: "bold" } },
        { times: 200, step: { action: "mark", type: "link" } },
        { times: 150, step: { action: "node" } },
        { operation: "text.type", times: "{times}", step: { action: "type", words: 1 } },
        { operation: "text.delete", times: "{times}", step: { action: "delete", characters: 5 } },
        { operation: "text.mark", times: "{times}", step: { action: "mark", type: "bold" } },
        {
            operation: "text.highlight",
            times: "{times}",
            step: { action: "mark", type: "highlight" },
        },
        { operation: "text.unmark", times: "{times}", step: { action: "unmark" } },
        { operation: "text.node", times: "{times}", step: { action: "node" } },
        { operation: "text.convert", times: "{times}", step: { action: "convert" } },
        { operation: "text.spans", times: 20, step: { action: "spans" } },
    ],
    then: [],
});

/** Edit a formatted text of about a million characters, the largest page a field holds. */
export const editLargeText = defineScenario({
    of: TextRope,
    interaction: textInteraction,
    name: "editLargeText",
    description: t`Build a text of about a million characters with marks and paragraphs, then type, delete, mark, unmark, insert and change nodes, and read spans in it.`,
    given: { examples: [] },
    parameters: { times: 200 },
    when: [
        { times: 500, step: { action: "append", characters: 2000 } },
        { times: 3000, step: { action: "mark", type: "bold" } },
        { times: 1000, step: { action: "node" } },
        { operation: "text.type", times: "{times}", step: { action: "type", words: 1 } },
        { operation: "text.delete", times: "{times}", step: { action: "delete", characters: 5 } },
        { operation: "text.mark", times: "{times}", step: { action: "mark", type: "bold" } },
        { operation: "text.unmark", times: "{times}", step: { action: "unmark" } },
        { operation: "text.node", times: "{times}", step: { action: "node" } },
        { operation: "text.convert", times: "{times}", step: { action: "convert" } },
        { operation: "text.spans", times: 3, step: { action: "spans" } },
    ],
    then: [],
});

/** Merge edits that parties made concurrently on one base. */
export const mergeConcurrentEdits = defineScenario({
    of: TextRope,
    interaction: textInteraction,
    name: "mergeConcurrentEdits",
    description: t`Merge the edits two and then twenty parties made concurrently on one base.`,
    given: { examples: [] },
    parameters: { times: 20 },
    when: [
        { times: 200, step: { action: "type", words: 9 } },
        {
            operation: "text.merge",
            times: "{times}",
            step: { action: "merge", parties: 2, changes: 50 },
        },
        {
            operation: "text.crowd",
            times: "{times}",
            step: { action: "merge", parties: 20, changes: 5 },
        },
    ],
    then: [],
});
