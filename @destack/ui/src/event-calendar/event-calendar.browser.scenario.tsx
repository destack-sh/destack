import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { EventCalendar } from "./event-calendar.tsx";
import { eventCalendarLaunchMonth, eventCalendarWorkWeek } from "./event-calendar.example.tsx";

/** The title of the calendar's shown range. */
const title = { css: "[data-slot=event-calendar-title] > :first-child" };

/** Move through a month's days with the arrows and to the next month with Page Down. */
export const eventCalendarMoveDaysWithKeys = defineScenario({
    of: EventCalendar,
    interaction: viewInteraction,
    name: "move-days-with-keys",
    description: "move through a month's days with the arrows, and a month on with Page Down",
    given: { examples: [eventCalendarLaunchMonth] },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { role: "button", name: "Wednesday, November 18, 2026" },
            },
            then: [{ observe: { kind: "text", target: title }, is: "November 2026" }],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [{ observe: { kind: "focused" }, is: "Thursday, November 19, 2026" }],
        },
        {
            step: { action: "press", key: "PageDown" },
            then: [
                { observe: { kind: "focused" }, is: "Saturday, December 19, 2026" },
                { observe: { kind: "text", target: title }, is: "December 2026" },
            ],
        },
    ],
});

/** Page a work week past its weekend. */
export const eventCalendarPageWorkWeeks = defineScenario({
    of: EventCalendar,
    interaction: viewInteraction,
    name: "page-work-weeks",
    description: "page a work week to the next, its weekend left out of the title and the columns",
    given: { examples: [eventCalendarWorkWeek] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "button", name: "Next" } },
            then: [
                { observe: { kind: "text", target: title }, is: "Nov 16 – 20, 2026" },
                {
                    observe: {
                        kind: "count",
                        target: { css: "[data-slot=event-calendar-column]" },
                    },
                    is: 5,
                },
            ],
        },
        {
            step: { action: "click", target: { role: "button", name: "Next" } },
            then: [{ observe: { kind: "text", target: title }, is: "Nov 23 – 27, 2026" }],
        },
    ],
});
