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
    given: { examples: [calendarLateOctober], globals: { locale: "de-AT" } },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { role: "button", name: "Freitag, 30. Oktober 2026" },
            },
            then: [
                { observe: { kind: "focused" }, is: "Freitag, 30. Oktober 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "Oktober 2026" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Samstag, 31. Oktober 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "Oktober 2026" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Sonntag, 1. November 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "November 2026" },
            ],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [
                { observe: { kind: "focused" }, is: "Sonntag, 8. November 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "November 2026" },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Montag, 2. November 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "November 2026" },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Sonntag, 8. November 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "November 2026" },
            ],
        },
        {
            step: { action: "press", key: "PageUp" },
            then: [
                { observe: { kind: "focused" }, is: "Donnerstag, 8. Oktober 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "Oktober 2026" },
            ],
        },
        {
            step: { action: "press", key: "ArrowUp" },
            then: [
                { observe: { kind: "focused" }, is: "Donnerstag, 1. Oktober 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "Oktober 2026" },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Mittwoch, 30. September 2026" },
                { observe: { kind: "text", target: { role: "heading" } }, is: "September 2026" },
            ],
        },
    ],
});
