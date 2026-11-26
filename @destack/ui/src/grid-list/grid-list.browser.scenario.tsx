import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { gridListReading } from "./grid-list.example.tsx";
import { GridList } from "./grid-list.tsx";

/** The rows of the reading list. */
const rows = { css: "[role=row]" };

/** The copy of the lifted row that follows the pointer. */
const preview = { css: "[data-slot=drag-preview]" };

/** Drag a row into another section with a mouse. */
export const gridListDragBetweenSections = defineScenario({
    of: GridList,
    interaction: viewInteraction,
    name: "dragBetweenSections",
    description: t`Press a row, drag it onto another section's heading, and drop it there.`,
    given: { examples: [gridListReading] },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: { text: "A Pattern Language" } } },
        { step: { action: "pointerDown" } },
        {
            step: { action: "pointerMove", target: { text: "Done" } },
            then: [{ observe: { kind: "visible", target: preview }, is: true }],
        },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "texts", target: rows },
                    is: [
                        "Read next",
                        "The Soul of a New Machine",
                        "Reading",
                        "Finding the Sweet Spot",
                        "The Mythical Man-Month",
                        "Done",
                        "A Pattern Language",
                        "Thinking Forth",
                    ],
                },
                { observe: { kind: "count", target: preview }, is: 0 },
            ],
        },
    ],
});
