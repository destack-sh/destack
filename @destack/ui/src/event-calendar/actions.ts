import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Show the next dates. */
export const showNext = defineAction({
    name: "calendar.next",
    title: t.context("action")`Next`,
    category: t.context("title")`Calendar`,
    keybinding: "j",
});

/** Show the previous dates. */
export const showPrevious = defineAction({
    name: "calendar.previous",
    title: t.context("action")`Previous`,
    category: t.context("title")`Calendar`,
    keybinding: "k",
});

/** Show today. */
export const showToday = defineAction({
    name: "calendar.today",
    title: t.context("action")`Today`,
    category: t.context("title")`Calendar`,
    keybinding: "t",
});

/** Show a day. */
export const showDay = defineAction({
    name: "calendar.day",
    title: t.context("action")`Show day`,
    category: t.context("title")`Calendar`,
    keybinding: "d",
});

/** Show some days. */
export const showDays = defineAction({
    name: "calendar.days",
    title: t.context("action")`Show days`,
    category: t.context("title")`Calendar`,
    keybinding: "x",
});

/** Show a week. */
export const showWeek = defineAction({
    name: "calendar.week",
    title: t.context("action")`Show week`,
    category: t.context("title")`Calendar`,
    keybinding: "w",
});

/** Show a month. */
export const showMonth = defineAction({
    name: "calendar.month",
    title: t.context("action")`Show month`,
    category: t.context("title")`Calendar`,
    keybinding: "m",
});

/** Show a year. */
export const showYear = defineAction({
    name: "calendar.year",
    title: t.context("action")`Show year`,
    category: t.context("title")`Calendar`,
    keybinding: "y",
});

/** Show an agenda. */
export const showAgenda = defineAction({
    name: "calendar.agenda",
    title: t.context("action")`Show agenda`,
    category: t.context("title")`Calendar`,
    keybinding: "a",
});

/** Copy the free time marked. */
export const copyAvailability = defineAction({
    name: "availability.copy",
    title: t.context("action")`Copy availability`,
    category: t.context("title")`Calendar`,
    keybinding: "mod+shift+c",
});

/** Clear the free time marked. */
export const clearAvailability = defineAction({
    name: "availability.clear",
    title: t.context("action")`Clear availability`,
    category: t.context("title")`Calendar`,
});

/** The action showing each view. */
export const VIEW_ACTIONS = {
    day: showDay,
    days: showDays,
    week: showWeek,
    month: showMonth,
    year: showYear,
    agenda: showAgenda,
} as const;
