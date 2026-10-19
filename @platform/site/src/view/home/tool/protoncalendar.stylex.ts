import * as style from "@destack/style";

/** The colours of the private calendar's events and calendars, light and dark as it ships. */
export const protonCalendar = style.defineConsts({
    /** An event's fill. */
    event: "light-dark(#ebe9e6, #2a2834)",
    /** The calendar of your own events. */
    mine: "#6d4aff",
    /** The calendar of the launch's events. */
    launch: "#da4bb3",
    /** The calendar of public holidays. */
    holidays: "#1aa36f",
    /** Today's date in the month and the week. */
    today: "light-dark(#6d4aff, #8a6eff)",
});
