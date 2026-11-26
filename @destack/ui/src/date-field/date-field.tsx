import type { PlainDate } from "@destack/schema";
import { type JSX, useLocale } from "@destack/view";
import {
    DateFormat,
    DateSegmentField,
    type DateSegmentFieldProperties,
    SEGMENT_FIELD_PROPERTIES,
} from "../date-segment/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of a date field besides its declared ones: its value, limits and attributes. */
export type DateFieldAttributes = Omit<
    DateSegmentFieldProperties<PlainDate>,
    keyof typeof SEGMENT_FIELD_PROPERTIES
>;

/** Render a date as one spinbutton per year, month and day in the locale's order. */
export const DateField = defineComponent({
    name: "dateField",
    properties: SEGMENT_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<DateFieldAttributes, typeof SEGMENT_FIELD_PROPERTIES>,
    ): JSX.Element {
        const format = new DateFormat(useLocale());

        return <DateSegmentField data-slot="date-field" {...properties} format={format} />;
    },
});

/** The properties of a date field, the native element's attributes included. */
export type DateFieldProperties = ComponentProperties<typeof DateField>;
