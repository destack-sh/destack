import { type JSX, omit, untrack, useEnvironment, useLocale } from "@destack/view";
import {
    DateSegmentField,
    type DateSegmentFieldProperties,
    DateTimeFormat,
    type HourCycle,
} from "../date-segment/index.ts";

/** The properties of a field of an instant in epoch milliseconds, with its element's attributes. */
export type DateTimeFieldProperties = DateSegmentFieldProperties<number> & {
    /** The zone its date and time are in, the person's by default. */
    readonly timeZone?: string | undefined;
    /** How many hours its clock counts to, the locale's by default. */
    readonly hourCycle?: HourCycle | undefined;
};

/** Render an instant as one spinbutton per part of its date and its time of day. */
export function DateTimeField(properties: DateTimeFieldProperties): JSX.Element {
    // read and write the instant in its zone
    const environment = useEnvironment();
    const format = new DateTimeFormat(
        useLocale(),
        () => properties.timeZone ?? environment.timeZone,
        untrack(() => properties.hourCycle),
    );
    const rest = omit(properties, "timeZone", "hourCycle");

    return <DateSegmentField data-slot="date-time-field" {...rest} format={format} />;
}
