import { t } from "@destack/locale";
import { type PlainDate, TimeZone } from "@destack/schema";
import {
    type Accessor,
    createControllableSignal,
    createMemo,
    untrack,
    type useLocale,
} from "@destack/view";
import type { CalendarResource } from "./source.ts";

/** A span of time marked free to share, a resource's where the day views show resources. */
export interface AvailabilitySpan {
    /** The first free instant, in epoch milliseconds. */
    readonly start: number;
    /** The instant the free time ends, after its start. */
    readonly end: number;
    /** The resource whose time it is, none without resources. */
    readonly resource?: string;
}

/** The options of a calendar's free time, which the calendar's options include. */
export interface CalendarAvailabilityOptions {
    /** The time marked free, controlled. */
    readonly availability?: readonly AvailabilitySpan[];
    /** The time marked free at first, uncontrolled. */
    readonly defaultAvailability?: readonly AvailabilitySpan[];
    /** Report the free time as the person marks or takes back spans. */
    readonly onAvailabilityChange?: (availability: readonly AvailabilitySpan[]) => void;
    /** The resources free spans may belong to. */
    readonly resources?: readonly CalendarResource[];
}

/** The time a person marks free to share. */
export class CalendarAvailability {
    /** The free spans: each resource's in order, joined where they meet. */
    readonly spans: Accessor<readonly AvailabilitySpan[]>;
    /** Replace the free spans. */
    readonly #setSpans: (spans: readonly AvailabilitySpan[]) => void;
    /** The options the free time follows. */
    readonly #options: CalendarAvailabilityOptions;
    /** The locale the text is written in. */
    readonly #locale: ReturnType<typeof useLocale>;
    /** The time zone the text writes times in. */
    readonly #timeZone: Accessor<string>;

    /** Follow the controlled free time, or keep the person's own, in a locale and a time zone. */
    constructor(
        options: CalendarAvailabilityOptions,
        locale: ReturnType<typeof useLocale>,
        timeZone: Accessor<string>,
    ) {
        // read the spans joined, whoever sets them
        const [spans, setSpans] = createControllableSignal<readonly AvailabilitySpan[]>({
            isControlled: () => options.availability !== undefined,
            value: () => options.availability ?? [],
            defaultValue: untrack(() => options.defaultAvailability ?? []),
            onChange: (availability) => options.onAvailabilityChange?.(availability),
        });
        this.spans = createMemo(() => joinSpans(spans()));
        this.#setSpans = setSpans;
        this.#options = options;
        this.#locale = locale;
        this.#timeZone = timeZone;
    }

    /** Mark a span free, joined with the free time of the same resource it meets or overlaps. */
    mark(span: AvailabilitySpan): void {
        this.#setSpans(joinSpans([...this.spans(), span]));
    }

    /** Take a free span back. */
    unmark(span: AvailabilitySpan): void {
        this.#setSpans(
            this.spans().filter(
                (each) =>
                    each.start !== span.start ||
                    each.end !== span.end ||
                    each.resource !== span.resource,
            ),
        );
    }

    /** Take all free time back. */
    clear(): void {
        this.#setSpans([]);
    }

    /** Write the free time as a line per day, under each resource's label, then the zone. */
    text(): string {
        // write a day's spans on a line, in the calendar's zone
        const locale = this.#locale;
        const zone = this.#timeZone();
        const lineOf = (day: PlainDate, spans: readonly AvailabilitySpan[]) => {
            const date = locale.plainDate(day, {
                weekday: "short",
                month: "short",
                day: "numeric",
            });
            const times = spans
                .map((span) =>
                    locale.dateRange(span.start, span.end, {
                        hour: "numeric",
                        minute: "2-digit",
                        timeZone: zone,
                    }),
                )
                .join(", ");

            return `${date}: ${times}`;
        };

        // group the spans by resource and by day, under each resource's label
        const byResource = Map.groupBy(this.spans(), (span) => span.resource);
        const blocks = [...byResource].map(([resource, spans]) => {
            // write the resource's days under its label, refusing a resource the calendar lacks
            const days = Map.groupBy(spans, (span) => TimeZone.date(zone, span.start));
            const lines = [...days].map(([day, each]) => lineOf(day, each));
            if (resource === undefined) {
                return lines;
            }
            const found = this.#options.resources?.find((each) => each.key === resource);
            if (found === undefined) {
                throw new TypeError(`calendar free time of a missing resource '${resource}'`);
            }

            return [found.label, ...lines];
        });

        return [...blocks.flat(), locale.render(t`Times in ${zone}`)].join("\n");
    }
}

/** Join free spans: each resource's in order, a span that meets the one before it merged in. */
function joinSpans(spans: readonly AvailabilitySpan[]): AvailabilitySpan[] {
    // order the spans by resource and start
    const ordered = spans.toSorted(
        (left, right) =>
            (left.resource ?? "").localeCompare(right.resource ?? "") || left.start - right.start,
    );

    // merge each span into the one before it where they meet
    const joined: AvailabilitySpan[] = [];
    for (const each of ordered) {
        const previous = joined.at(-1);
        if (
            previous !== undefined &&
            previous.resource === each.resource &&
            each.start <= previous.end
        ) {
            joined[joined.length - 1] = { ...previous, end: Math.max(previous.end, each.end) };
        } else {
            joined.push(each);
        }
    }

    return joined;
}
