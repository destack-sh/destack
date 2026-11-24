import type { PlainDate } from "@destack/schema";
import { type JSX, useLocale } from "@destack/view";
import {
    DateFormat,
    DateSegmentField,
    type DateSegmentFieldProperties,
} from "../date-segment/index.ts";

/** The properties of a date field, the native element's attributes included. */
export type DateFieldProperties = DateSegmentFieldProperties<PlainDate>;

/** Render a date as one spinbutton per year, month and day in the locale's order. */
export function DateField(properties: DateFieldProperties): JSX.Element {
    const format = new DateFormat(useLocale());

    return <DateSegmentField data-slot="date-field" {...properties} format={format} />;
}
