import type { PlainTime } from "@destack/schema";
import { type JSX, omit, untrack, useLocale } from "@destack/view";
import {
    DateSegmentField,
    type DateSegmentFieldProperties,
    TIME_SEGMENT_FIELD_PROPERTIES,
    TimeFormat,
} from "../date-segment/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of a time field besides its declared ones: its value, limits and attributes. */
export type TimeFieldAttributes = Omit<
    DateSegmentFieldProperties<PlainTime>,
    keyof typeof TIME_SEGMENT_FIELD_PROPERTIES
>;

/** Render a time of day as one spinbutton per hour, minute and day period. */
export const TimeField = defineComponent({
    name: "timeField",
    properties: TIME_SEGMENT_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<TimeFieldAttributes, typeof TIME_SEGMENT_FIELD_PROPERTIES>,
    ): JSX.Element {
        // lay out the clock
        const hourCycle = untrack(() => properties.hourCycle);
        const format = new TimeFormat(useLocale(), hourCycle);
        const rest = omit(properties, "hourCycle");

        return <DateSegmentField data-slot="time-field" {...rest} format={format} />;
    },
});

/** The properties of a time field, the native element's attributes included. */
export type TimeFieldProperties = ComponentProperties<typeof TimeField>;
