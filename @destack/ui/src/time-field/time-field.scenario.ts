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
    given: { examples: [timeFieldReminder], globals: { locale: "en-US" } },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "spinbutton", name: "Hour" } },
            then: [
                { observe: { kind: "focused" }, is: "Hour" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Reminder" } },
                    is: "08:30 AM",
                },
            ],
        },
        {
            step: { action: "press", key: "9" },
            then: [
                { observe: { kind: "focused" }, is: "Minute" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Reminder" } },
                    is: "09:30 AM",
                },
            ],
        },
        {
            step: { action: "press", key: "4" },
            then: [
                { observe: { kind: "focused" }, is: "Minute" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Reminder" } },
                    is: "09:04 AM",
                },
            ],
        },
        {
            step: { action: "press", key: "5" },
            then: [
                { observe: { kind: "focused" }, is: "AM/PM" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Reminder" } },
                    is: "09:45 AM",
                },
            ],
        },
        {
            step: { action: "press", key: "p" },
            then: [
                { observe: { kind: "focused" }, is: "AM/PM" },
                {
                    observe: { kind: "text", target: { role: "group", name: "Reminder" } },
                    is: "09:45 PM",
                },
            ],
        },
    ],
});
