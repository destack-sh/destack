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
    given: { examples: [dateFieldBirthday], environment: { locale: "en-US" } },
    when: [
        { action: "focus", target: { role: "spinbutton", name: "Month" } },
        { action: "press", key: "1" },
        { action: "press", key: "0" },
        { action: "press", key: "7" },
        { action: "press", key: "1" },
        { action: "press", key: "9" },
        { action: "press", key: "9" },
        { action: "press", key: "0" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            date: { kind: "text", target: { role: "group", name: "Birthday" } },
        },
        each: [
            { focused: "Month", date: "––/––/––––" },
            { focused: "Month", date: "01/––/––––" },
            { focused: "Day", date: "10/––/––––" },
            { focused: "Year", date: "10/07/––––" },
            { focused: "Year", date: "10/07/1" },
            { focused: "Year", date: "10/07/19" },
            { focused: "Year", date: "10/07/199" },
            { focused: "Year", date: "10/07/1990" },
        ],
    },
});

/** Step and clear segments with the keys. */
export const dateFieldStepSegments = defineScenario({
    of: DateField,
    interaction: viewInteraction,
    name: "step-segments",
    description:
        "step a segment with the up and down arrow keys, wrapping at its bounds, move along the segments with the side arrow keys and clear one with Backspace",
    given: { examples: [dateFieldArchived], environment: { locale: "en-US" } },
    when: [
        { action: "focus", target: { role: "spinbutton", name: "Month" } },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "Backspace" },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            date: { kind: "text", target: { role: "group", name: "Archived" } },
        },
        each: [
            { focused: "Month", date: "10/07/2026" },
            { focused: "Month", date: "11/07/2026" },
            { focused: "Month", date: "12/07/2026" },
            { focused: "Month", date: "01/07/2026" },
            { focused: "Day", date: "01/07/2026" },
            { focused: "Day", date: "01/––/2026" },
            { focused: "Month", date: "01/––/2026" },
        ],
    },
});
