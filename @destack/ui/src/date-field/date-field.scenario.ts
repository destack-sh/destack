import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { dateFieldArchived, dateFieldBirthday } from "./date-field.example.tsx";
import { DateField } from "./date-field.tsx";

/** Type a date digit by digit. */
export const dateFieldTypeDigits = defineScenario({
    of: DateField,
    interaction: viewInteraction,
    name: "type-digits",
    description:
        "type a date digit by digit, moving on to the next segment once a segment takes no more digits",
    given: { examples: [dateFieldBirthday], globals: { locale: "en-US" } },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "spinbutton", name: "Month" } },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "––/––/––––",
                },
            ],
        },
        {
            step: { action: "press", key: "1" },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "01/––/––––",
                },
            ],
        },
        {
            step: { action: "press", key: "0" },
            then: [
                { observe: { kind: "focused" }, is: "Day" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/––/––––",
                },
            ],
        },
        {
            step: { action: "press", key: "7" },
            then: [
                { observe: { kind: "focused" }, is: "Year" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/07/––––",
                },
            ],
        },
        {
            step: { action: "press", key: "1" },
            then: [
                { observe: { kind: "focused" }, is: "Year" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/07/1",
                },
            ],
        },
        {
            step: { action: "press", key: "9" },
            then: [
                { observe: { kind: "focused" }, is: "Year" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/07/19",
                },
            ],
        },
        {
            step: { action: "press", key: "9" },
            then: [
                { observe: { kind: "focused" }, is: "Year" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/07/199",
                },
            ],
        },
        {
            step: { action: "press", key: "0" },
            then: [
                { observe: { kind: "focused" }, is: "Year" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Birthday" } },
                    is: "10/07/1990",
                },
            ],
        },
    ],
});

/** Step and clear segments with the keys. */
export const dateFieldStepSegments = defineScenario({
    of: DateField,
    interaction: viewInteraction,
    name: "step-segments",
    description:
        "step a segment with the up and down arrow keys, wrapping at its bounds, move along the segments with the side arrow keys and clear one with Backspace",
    given: { examples: [dateFieldArchived], globals: { locale: "en-US" } },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "spinbutton", name: "Month" } },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "10/07/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "11/07/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "12/07/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "01/07/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Day" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "01/07/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "Backspace" },
            then: [
                { observe: { kind: "focused" }, is: "Day" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "01/––/2026",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Month" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Archived" } },
                    is: "01/––/2026",
                },
            ],
        },
    ],
});
