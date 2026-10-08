import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { datePickerTrip } from "./date-picker.example.tsx";
import { DatePicker } from "./date-picker.tsx";

/** Focus the calendar's day as it opens. */
export const datePickerFocusDayOnOpen = defineScenario({
    of: DatePicker,
    interaction: viewInteraction,
    name: "focus-day-on-open",
    description:
        "move the focus onto the calendar's focused day, the range's start, as the calendar opens",
    given: { examples: [datePickerTrip], environment: { locale: "en-US" } },
    when: [{ action: "click", target: { role: "button", name: "Pick a date" } }],
    then: {
        observe: { focused: { kind: "focused" } },
        each: [{ focused: "Monday, October 12, 2026" }],
    },
});
