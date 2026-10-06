import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { calendarLateOctober } from "./calendar.example.tsx";
import { Calendar } from "./calendar.tsx";

/** Move a calendar's focused day with the keys in Austria. */
export const calendarMoveFocusedDay = defineScenario({
    of: Calendar,
    interaction: viewInteraction,
    name: "move-focused-day",
    description:
        "move the focused day with arrows, Home, End, Page Up and Page Down, crossing months",
    given: { examples: [calendarLateOctober], environment: { locale: "de-AT" } },
    when: [
        { action: "focus", target: { role: "button", name: "Freitag, 30. Oktober 2026" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "Home" },
        { action: "press", key: "End" },
        { action: "press", key: "PageUp" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "ArrowLeft" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            month: { kind: "text", target: { role: "heading" } },
        },
        each: [
            { focused: "Freitag, 30. Oktober 2026", month: "Oktober 2026" },
            { focused: "Samstag, 31. Oktober 2026", month: "Oktober 2026" },
            { focused: "Sonntag, 1. November 2026", month: "November 2026" },
            { focused: "Sonntag, 8. November 2026", month: "November 2026" },
            { focused: "Montag, 2. November 2026", month: "November 2026" },
            { focused: "Sonntag, 8. November 2026", month: "November 2026" },
            { focused: "Donnerstag, 8. Oktober 2026", month: "Oktober 2026" },
            { focused: "Donnerstag, 1. Oktober 2026", month: "Oktober 2026" },
            { focused: "Mittwoch, 30. September 2026", month: "September 2026" },
        ],
    },
});
