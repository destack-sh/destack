import { t } from "@destack/locale";
import { defineAction } from "@destack/object";

/** Show the next dates. */
export const showNext = defineAction({
    name: "calendar.next",
    title: t`Next`,
    category: t`Calendar`,
    keybinding: "j",
});

/** Show the previous dates. */
export const showPrevious = defineAction({
    name: "calendar.previous",
    title: t`Previous`,
    category: t`Calendar`,
    keybinding: "k",
});

/** Show today. */
export const showToday = defineAction({
    name: "calendar.today",
    title: t`Today`,
    category: t`Calendar`,
    keybinding: "t",
});

/** Show a day. */
export const showDay = defineAction({
    name: "calendar.day",
    title: t`Show day`,
    category: t`Calendar`,
    keybinding: "d",
});

/** Show some days. */
export const showDays = defineAction({
    name: "calendar.days",
    title: t`Show days`,
    category: t`Calendar`,
    keybinding: "x",
});

/** Show a week. */
export const showWeek = defineAction({
    name: "calendar.week",
    title: t`Show week`,
    category: t`Calendar`,
    keybinding: "w",
});

/** Show a month. */
export const showMonth = defineAction({
    name: "calendar.month",
    title: t`Show month`,
    category: t`Calendar`,
    keybinding: "m",
});

/** Show a year. */
export const showYear = defineAction({
    name: "calendar.year",
    title: t`Show year`,
    category: t`Calendar`,
    keybinding: "y",
});

/** Show an agenda. */
export const showAgenda = defineAction({
    name: "calendar.agenda",
    title: t`Show agenda`,
    category: t`Calendar`,
    keybinding: "a",
});

/** Copy the free time marked. */
export const copyAvailability = defineAction({
    name: "availability.copy",
    title: t`Copy availability`,
    category: t`Calendar`,
    keybinding: "mod+shift+c",
});

/** Clear the free time marked. */
export const clearAvailability = defineAction({
    name: "availability.clear",
    title: t`Clear availability`,
    category: t`Calendar`,
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
