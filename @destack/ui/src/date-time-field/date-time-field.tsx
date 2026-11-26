import { type JSX, omit, untrack, useEnvironment, useLocale } from "@destack/view";
import {
    DateSegmentField,
    type DateSegmentFieldProperties,
    DateTimeFormat,
    TIME_SEGMENT_FIELD_PROPERTIES,
} from "../date-segment/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of an instant's field besides its declared ones: its value, zone and element. */
export type DateTimeFieldAttributes = Omit<
    DateSegmentFieldProperties<number>,
    keyof typeof TIME_SEGMENT_FIELD_PROPERTIES
> & {
    /** The zone its date and time are in, the person's by default. */
    readonly timeZone?: string | undefined;
};

/** Render an instant as one spinbutton per part of its date and its time of day. */
export const DateTimeField = defineComponent({
    name: "dateTimeField",
    properties: TIME_SEGMENT_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<DateTimeFieldAttributes, typeof TIME_SEGMENT_FIELD_PROPERTIES>,
    ): JSX.Element {
        // read and write the instant in its zone
        const environment = useEnvironment();
        const format = new DateTimeFormat(
            useLocale(),
            () => properties.timeZone ?? environment.timeZone,
            untrack(() => properties.hourCycle),
        );
        const rest = omit(properties, "timeZone", "hourCycle");

        return <DateSegmentField data-slot="date-time-field" {...rest} format={format} />;
    },
});

/** The properties of a field of an instant in epoch milliseconds, with its element's attributes. */
export type DateTimeFieldProperties = ComponentProperties<typeof DateTimeField>;
