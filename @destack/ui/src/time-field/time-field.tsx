import type { PlainTime } from "@destack/schema";
import { type JSX, omit, untrack, useLocale } from "@destack/view";
import {
    DateSegmentField,
    type DateSegmentFieldProperties,
    type HourCycle,
    TimeFormat,
} from "../date-segment/index.ts";

/** The properties of a time field, the native element's attributes included. */
export type TimeFieldProperties = DateSegmentFieldProperties<PlainTime> & {
    /** How many hours its clock counts to, the locale's by default. */
    readonly hourCycle?: HourCycle | undefined;
};

/** Render a time of day as one spinbutton per hour, minute and day period. */
export function TimeField(properties: TimeFieldProperties): JSX.Element {
    // lay out the clock
    const hourCycle = untrack(() => properties.hourCycle);
    const format = new TimeFormat(useLocale(), hourCycle);
    const rest = omit(properties, "hourCycle");

    return <DateSegmentField data-slot="time-field" {...rest} format={format} />;
}
