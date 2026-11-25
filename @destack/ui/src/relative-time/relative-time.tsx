import type { RelativeFormat } from "@destack/locale";
import { Duration } from "@destack/schema";
import * as style from "@destack/style";
import { type JSX, omit, untrack, useEnvironment, useLocale, useNow } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

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

/** The properties of a relative time, the native element's attributes included. */
export type RelativeTimeProperties = Omit<
    JSX.TimeHTMLAttributes<HTMLTimeElement>,
    "class" | "datetime" | "children"
> &
    ElementPartProperties & {
        /** The moment, in milliseconds. */
        readonly value: number;
        /** The time before or after now past which the date is written, 30 days by default. */
        readonly threshold?: Duration;
        /** How long the words are, long by default, such as 3 days ago or 3d ago. */
        readonly length?: RelativeFormat["style"];
        /** The smallest unit written, below which it is now, minutes by default. */
        readonly precision?: RelativeFormat["precision"];
        /** How the date is written past the threshold, its medium style by default. */
        readonly format?: Intl.DateTimeFormatOptions;
        /** The time zone of the date, the environment's by default. */
        readonly timeZone?: string;
        /** Render the words, as they are by default. */
        readonly children?: (words: string) => JSX.Element;
    };

/** Render a moment relative to now in the reader's language, moving on with the clock. */
export function RelativeTime(properties: RelativeTimeProperties): JSX.Element {
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
        const isNear = current !== undefined && Math.abs(properties.value - current) < threshold;

        return isNear
            ? locale.relative(properties.value, current, {
                  style: properties.length ?? "long",
                  precision: properties.precision ?? "minute",
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
}
