import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { sortableLongList } from "./sortable.example.tsx";
import { Sortable } from "./sortable.tsx";

/** The live region announcing each step of a drag. */
const status = { role: "status" };

/** Step a lifted item past the rows a virtual list mounts, which scrolls along with it. */
export const sortableStepPastWindow = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "stepPastWindow",
    description: t`Lift the first of a thousand tasks and step it past the sixteen rows the list mounts, the list scrolling along.`,
    given: { examples: [sortableLongList] },
    parameters: {},
    when: [
        { step: { action: "focus", target: { role: "button", name: "Move", nth: 0 } } },
        { step: { action: "press", key: " " } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        {
            step: { action: "pause", milliseconds: 50 },
            then: [
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Task 1⁩ moved to position 17 of 1,000 in ⁨Tasks⁩.",
                },
            ],
        },
        {
            step: { action: "press", key: " " },
            then: [
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Task 1⁩ dropped at position 17 of 1,000 in ⁨Tasks⁩.",
                },
            ],
        },
    ],
});

/** Cancel a lifted item stepped past the rows in view, the list scrolling back to it. */
export const sortableCancelPastWindow = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "cancelPastWindow",
    description: t`Lift the first of a thousand tasks, step it past the rows the list mounts, and cancel with Escape.`,
    given: { examples: [sortableLongList] },
    parameters: {},
    when: [
        { step: { action: "focus", target: { role: "button", name: "Move", nth: 0 } } },
        { step: { action: "press", key: " " } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "ArrowDown" } },
        { step: { action: "press", key: "Escape" } },
        {
            step: { action: "pause", milliseconds: 50 },
            then: [
                {
                    observe: { kind: "text", target: status },
                    is: "Moving ⁨Task 1⁩ was cancelled. It returned to position 1 of 1,000 in ⁨Tasks⁩.",
                },
                {
                    observe: {
                        kind: "visible",
                        target: { css: "[data-slot=sortable-item]", nth: 0 },
                    },
                    is: true,
                },
                {
                    observe: { kind: "text", target: { css: "[data-slot=sortable-item]", nth: 0 } },
                    is: "Task 1",
                },
            ],
        },
    ],
});

/** Drag an item by pointer while the list scrolls under it, the item staying mounted. */
export const sortableDragPastWindow = defineScenario({
    of: Sortable,
    interaction: viewInteraction,
    name: "dragPastWindow",
    description: t`Press the first of a thousand tasks, wheel the list a hundred rows on, and drop it there.`,
    given: { examples: [sortableLongList] },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: { role: "button", name: "Move", nth: 0 } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pointerMove", target: { role: "button", name: "Move", nth: 3 } } },
        { step: { action: "wheel", deltaY: 3600 } },
        {
            step: { action: "pause", milliseconds: 100 },
            then: [
                {
                    observe: { kind: "text", target: { css: "[data-slot=sortable-item]", nth: 0 } },
                    is: "Task 1",
                },
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Task 1⁩ moved to position 104 of 1,000 in ⁨Tasks⁩.",
                },
            ],
        },
        {
            step: { action: "pointerUp" },
            then: [
                {
                    observe: { kind: "text", target: status },
                    is: "⁨Task 1⁩ dropped at position 104 of 1,000 in ⁨Tasks⁩.",
                },
            ],
        },
    ],
});
