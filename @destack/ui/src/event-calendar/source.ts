import type { Hue } from "@destack/color";

/** How a calendar's events show: in full, or as busy time without their details. */
export type CalendarSourceDisplay = "events" | "busy";

/** A calendar events belong to, after Google Calendar's list: a color, shown or hidden. */
export interface CalendarSource {
    /** The calendar's key, which its events name as their source. */
    readonly key: string;
    /** The calendar's label in the list. */
    readonly label: string;
    /** The color its events take, unless they have their own. */
    readonly color?: Hue;
    /** How its events show, in full by default, as busy time after Notion Calendar's overlay. */
    readonly display?: CalendarSourceDisplay;
}

/** A person or thing events belong to, which the day views give columns, after FullCalendar. */
export interface CalendarResource {
    /** The resource's key, which its events name as their resource. */
    readonly key: string;
    /** The resource's label above its columns. */
    readonly label: string;
}
