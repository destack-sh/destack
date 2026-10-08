import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { timeFieldReminder } from "./time-field.example.tsx";
import { TimeField } from "./time-field.tsx";

/** Type a time and choose its day period by letter. */
export const timeFieldChooseDayPeriod = defineScenario({
    of: TimeField,
    interaction: viewInteraction,
    name: "choose-day-period",
    description:
        "type the hour and minute digit by digit and choose the afternoon by its letter on a 12-hour clock",
    given: { examples: [timeFieldReminder], environment: { locale: "en-US" } },
    when: [
        { action: "focus", target: { role: "spinbutton", name: "Hour" } },
        { action: "press", key: "9" },
        { action: "press", key: "4" },
        { action: "press", key: "5" },
        { action: "press", key: "p" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            time: { kind: "text", target: { role: "group", name: "Reminder" } },
        },
        each: [
            { focused: "Hour", time: "08:30 AM" },
            { focused: "Minute", time: "09:30 AM" },
            { focused: "Minute", time: "09:04 AM" },
            { focused: "AM/PM", time: "09:45 AM" },
            { focused: "AM/PM", time: "09:45 PM" },
        ],
    },
});
