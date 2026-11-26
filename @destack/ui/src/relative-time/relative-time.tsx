import { field } from "@destack/object";
import { t } from "@destack/locale";
import { Duration } from "@destack/schema";
import * as style from "@destack/style";
import { type JSX, omit, untrack, useEnvironment, useLocale, useNow } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The time after which a date says more than how long ago it is, as GitHub's relative-time. */
const THRESHOLD: Duration = { days: 30 };

/** How a date is written past the threshold. */
const DATE: Intl.DateTimeFormatOptions = { dateStyle: "medium" };

/** How the moment is written in full, on hover. */
const FULL: Intl.DateTimeFormatOptions = { dateStyle: "full", timeStyle: "short" };

/** The styles of a relative time. */
const styles = style.create({
    time: {
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
});

/** The properties people change in a relative time's controls and inspector, named as `Intl`'s. */
const RELATIVE_TIME_PROPERTIES = {
    length: field
        .enum({
            long: {
                title: t.context("option")`Long`,
                description: t`In full words.`,
            },
            short: {
                title: t.context("option")`Short`,
                description: t`In shortened words.`,
            },
            narrow: {
                title: t.context("option")`Narrow`,
                description: t`In the fewest letters.`,
            },
        })
        .labeled(t.context("label")`Length`)
        .described(t`How long the words are.`)
        .default("long"),
    precision: field
        .enum({
            second: { title: t.context("option")`Second` },
            minute: { title: t.context("option")`Minute` },
            hour: { title: t.context("option")`Hour` },
            day: { title: t.context("option")`Day` },
        })
        .labeled(t.context("label")`Precision`)
        .described(t`The smallest unit written, below which it is now.`)
        .default("minute"),
};

/** The properties of a relative time besides its declared ones: its moment and date format. */
export type RelativeTimeAttributes = Omit<
    JSX.TimeHTMLAttributes<HTMLTimeElement>,
    "class" | "datetime" | "children"
> &
    ElementPartProperties & {
        /** The moment, in milliseconds. */
        readonly value: number;
        /** The time before or after now past which the date is written, 30 days by default. */
        readonly threshold?: Duration;
        /** How the date is written past the threshold, its medium style by default. */
        readonly format?: Intl.DateTimeFormatOptions;
        /** The time zone of the date, the environment's by default. */
        readonly timeZone?: string;
        /** Render the words, as they are by default. */
        readonly children?: (words: string) => JSX.Element;
    };

/** Render a moment relative to now in the reader's language, moving on with the clock. */
export const RelativeTime = defineComponent({
    name: "relativeTime",
    properties: RELATIVE_TIME_PROPERTIES,
    render(
        properties: RenderProperties<RelativeTimeAttributes, typeof RELATIVE_TIME_PROPERTIES>,
    ): JSX.Element {
        // move on as often as the words may change
        const locale = useLocale();
        const environment = useEnvironment();
        const isExact = untrack(() => properties.precision === "second");
        const now = useNow(isExact ? { seconds: 1 } : { minutes: 1 });
        const zone = (): string => properties.timeZone ?? environment.timeZone;
        const date = (format: Intl.DateTimeFormatOptions): string =>
            locale.date(properties.value, { ...format, timeZone: zone() });
        const words = (): string => {
            // write the date before the clock is read, and past the threshold
            const current = now();
            const threshold = Duration.milliseconds(properties.threshold ?? THRESHOLD);
            const isNear =
                current !== undefined && Math.abs(properties.value - current) < threshold;

            return isNear
                ? locale.relative(properties.value, current, {
                      style: properties.length,
                      precision: properties.precision,
                  })
                : date(properties.format ?? DATE);
        };
        const rest = omit(
            properties,
            "value",
            "threshold",
            "length",
            "precision",
            "format",
            "timeZone",
            "children",
        );

        return renderPart("time", "relative-time", rest, styles.time, {
            get datetime() {
                return new Date(properties.value).toISOString();
            },
            get title() {
                return date(FULL);
            },
            get children() {
                return properties.children?.(words()) ?? words();
            },
        });
    },
});

/** The properties of a relative time, the native element's attributes included. */
export type RelativeTimeProperties = ComponentProperties<typeof RelativeTime>;
