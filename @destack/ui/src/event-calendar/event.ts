import type { AccentPreset } from "@destack/color";
import {
    type DateSpan,
    Duration,
    PlainDate,
    PlainTime,
    Recurrence,
    type RecurrenceRule,
    TimeZone,
    type Week,
} from "@destack/schema";

/** The milliseconds of a day, from which a timed event shows among the all-day ones. */
const DAY = Duration.milliseconds({ days: 1 });

/** The milliseconds of a minute. */
const MINUTE = 60_000;

/** The minutes of a day on the wall clock. */
const DAY_MINUTES = 24 * 60;

/** The days two zones' dates of one instant differ by at most, their offsets 26 hours apart. */
const ZONE_DAYS = 2;

/** The first day of epoch time, which wall minutes count from. */
const EPOCH: PlainDate = "1970-01-01";

/** The milliseconds of an hour, which a whole-day event moved into the hours runs for. */
const HOUR = Duration.milliseconds({ hours: 1 });

/** How an event shows, after FullCalendar: as an event, or as shading behind the others. */
export type CalendarEventDisplay = "auto" | "background";

/** A change to one repeat of an event, after JSCalendar's recurrence overrides. */
export type RecurrenceOverride<Time> =
    | {
          /** The repeat does not happen. */
          readonly excluded: true;
      }
    | {
          /** The repeat's start. */
          readonly start: Time;
          /** The repeat's end. */
          readonly end: Time;
      };

/** The fields every event has, over whole days or over a time. */
interface CalendarEventFields<Time> {
    /** The event's key. */
    readonly key: string;
    /** The event's title. */
    readonly title: string;
    /** The start: the first day, or an instant in epoch milliseconds. */
    readonly start: Time;
    /** The end: the last day, or an instant at or after the start. */
    readonly end: Time;
    /** The event's color, blue by default. */
    readonly color?: AccentPreset;
    /** How the event shows, as an event by default. */
    readonly display?: CalendarEventDisplay;
    /** The rules it repeats by, from its first day, once when absent. */
    readonly recurrenceRules?: readonly RecurrenceRule[];
    /** The changes to single repeats, by the date each repeat would start on. */
    readonly recurrenceOverrides?: Readonly<Record<PlainDate, RecurrenceOverride<Time>>>;
    /** The key of the calendar it belongs to, whose color and display it takes. */
    readonly source?: string;
    /** The key of the resource it belongs to, such as a person, whose column it shows in. */
    readonly resource?: string;
}

/** An event after JSCalendar: over whole days from dates, or over a time from instants. */
export type CalendarEvent =
    | CalendarEventFields<PlainDate>
    | (CalendarEventFields<number> & {
          /** The IANA time zone its wall times repeat and move in, the calendar's by default. */
          readonly timeZone?: string;
      });

/** An event before its caller keys it, such as a copy the person drops or a draft they type. */
export type CalendarEventDraft = CalendarEvent extends infer Each
    ? Each extends CalendarEvent
        ? Omit<Each, "key">
        : never
    : never;

/** An event over a time, its start and end instants. */
export type TimedCalendarEvent = Extract<CalendarEvent, { readonly start: number }>;

/** One time an event happens: the event itself, or one of its repeats. */
export interface CalendarOccurrence {
    /** The event as it happens this time, keyed by its repeat. */
    readonly event: CalendarEvent;
    /** The repeating event, none for an event that happens once. */
    readonly of: CalendarEvent | undefined;
    /** The date the repeat would start on, which keys its override. */
    readonly recurrenceId: PlainDate;
}

/** The verbs of events. */
export const CalendarEvent = {
    /** Report whether an event runs over a time rather than whole days. */
    isTimed(event: CalendarEvent): event is TimedCalendarEvent {
        return typeof event.start === "number";
    },

    /** Report whether an event shows among the all-day ones: over whole days, or a day or more. */
    isAllDay(event: CalendarEvent): boolean {
        return !CalendarEvent.isTimed(event) || event.end - event.start >= DAY;
    },

    /** Report whether an event shows as shading behind the others. */
    isBackground(event: CalendarEvent): boolean {
        return event.display === "background";
    },

    /** Read an event's days in a zone, its last day included. */
    span(event: CalendarEvent, timeZone: string): DateSpan {
        // read a timed event's days, or an all-day event's dates
        if (CalendarEvent.isTimed(event)) {
            const last = Math.max(event.end - 1, event.start);

            return {
                key: event.key,
                start: TimeZone.date(timeZone, event.start),
                end: TimeZone.date(timeZone, last),
            };
        } else {
            return { key: event.key, start: event.start, end: event.end };
        }
    },

    /** Move an event by wall-clock minutes in its zone, whole days for one over whole days. */
    shift(event: CalendarEvent, minutes: number, timeZone: string): CalendarEvent {
        return shifterOf(event, timeZone)(minutes);
    },

    /** Read a series' repeat on a date where its rules place it, before any override. */
    repeatOn(event: CalendarEvent, date: PlainDate, timeZone: string): CalendarEvent {
        // move the series to the date at its wall times
        const days = PlainDate.daysUntil(CalendarEvent.first(event, timeZone), date);

        return repeatOf(event.key, shifterOf(event, timeZone)(days * DAY_MINUTES), date);
    },

    /** Read the day a series starts on in the zone its wall times repeat in. */
    first(event: CalendarEvent, timeZone: string): PlainDate {
        return CalendarEvent.isTimed(event)
            ? TimeZone.date(event.timeZone ?? timeZone, event.start)
            : event.start;
    },

    /** Report whether an event touches a range of dates in a zone. */
    isIn(
        event: CalendarEvent,
        range: { readonly from: PlainDate; readonly to: PlainDate },
        timeZone: string,
    ): boolean {
        const span = CalendarEvent.span(event, timeZone);

        return (
            PlainDate.compare(span.end, range.from) >= 0 &&
            PlainDate.compare(span.start, range.to) <= 0
        );
    },

    /** List the times an event happens in a range of dates: once, or each repeat. */
    occurrences(
        event: CalendarEvent,
        range: { readonly from: PlainDate; readonly to: PlainDate },
        timeZone: string,
        week: Week,
    ): CalendarOccurrence[] {
        // take an event that happens once while it touches the range
        const span = CalendarEvent.span(event, timeZone);
        const rules = event.recurrenceRules;
        if (rules === undefined || rules.length === 0) {
            return CalendarEvent.isIn(event, range, timeZone)
                ? [{ event, of: undefined, recurrenceId: span.start }]
                : [];
        }

        // list the repeats that start near the range, and those an override moves into it
        const first = CalendarEvent.first(event, timeZone);
        const shift = shifterOf(event, timeZone);
        const isLocal = !CalendarEvent.isTimed(event) || (event.timeZone ?? timeZone) === timeZone;
        const length = PlainDate.daysUntil(span.start, span.end);
        const margin = isLocal ? 0 : ZONE_DAYS;
        const near = Recurrence.dates(
            rules,
            first,
            PlainDate.add(range.from, { days: -length - margin }),
            PlainDate.add(range.to, { days: margin }),
            week,
        );
        const isNear = new Set(near);
        const moved = Object.keys(event.recurrenceOverrides ?? {}).filter((date) => {
            // check the override's times first, and that a rule repeats on its date last
            const override = event.recurrenceOverrides?.[date];

            return (
                override !== undefined &&
                !isNear.has(date) &&
                !("excluded" in override) &&
                CalendarEvent.isIn(CalendarEvent.retime(event, override), range, timeZone) &&
                Recurrence.dates(rules, first, date, date, week).length > 0
            );
        });

        // take each repeat in the range but an excluded one, at its date or its override's times
        return [...near, ...moved].flatMap((date): CalendarOccurrence[] => {
            // move the event to the repeat's date, keyed by it
            const override = event.recurrenceOverrides?.[date];
            const repeat = repeatOf(
                event.key,
                shift(PlainDate.daysUntil(first, date) * DAY_MINUTES),
                date,
            );
            const retimed =
                override === undefined || "excluded" in override
                    ? repeat
                    : CalendarEvent.retime(repeat, override);
            const isShown =
                override === undefined
                    ? (isLocal && PlainDate.compare(date, range.from) >= 0) ||
                      CalendarEvent.isIn(retimed, range, timeZone)
                    : !("excluded" in override) && CalendarEvent.isIn(retimed, range, timeZone);

            return isShown ? [{ event: retimed, of: event, recurrenceId: date }] : [];
        });
    },

    /** Read the wall minutes between two events' starts, or their days where one is whole-day. */
    travel(from: CalendarEvent, to: CalendarEvent, timeZone: string): number {
        // count the wall minutes in the zone the first event's times are in
        if (CalendarEvent.isTimed(from) && CalendarEvent.isTimed(to)) {
            const zone = from.timeZone ?? timeZone;

            return wallMinutesOf(to.start, zone) - wallMinutesOf(from.start, zone);
        }
        // else count the days between their first days
        else {
            const days = PlainDate.daysUntil(
                CalendarEvent.span(from, timeZone).start,
                CalendarEvent.span(to, timeZone).start,
            );

            return days * DAY_MINUTES;
        }
    },

    /** Move an event as another moved: by its wall minutes, or its days for a whole-day event. */
    along(
        event: CalendarEvent,
        from: CalendarEvent,
        to: CalendarEvent,
        timeZone: string,
    ): CalendarEvent {
        // move a timed event by the travel, and a whole-day one by the days the start moved
        const travel = CalendarEvent.travel(from, to, timeZone);
        const minutes = CalendarEvent.isTimed(event)
            ? travel
            : Math.floor(travel / DAY_MINUTES) * DAY_MINUTES;
        const first = CalendarEvent.first(event, timeZone);

        return CalendarEvent.shiftSeries(event, minutes, timeZone, first);
    },

    /** Move a series by wall minutes, its rules with one of its dates, its overrides alike. */
    shiftSeries(
        event: CalendarEvent,
        minutes: number,
        timeZone: string,
        anchor: PlainDate,
    ): CalendarEvent {
        // move the anchor date by the days the series' start moves on the wall clock
        const moved = shifterOf(event, timeZone)(minutes);
        const days = PlainDate.daysUntil(
            CalendarEvent.first(event, timeZone),
            CalendarEvent.first(moved, timeZone),
        );
        const dated = (date: string) => PlainDate.add(date, { days });
        const rules =
            event.recurrenceRules === undefined
                ? {}
                : {
                      recurrenceRules: Recurrence.move(
                          event.recurrenceRules,
                          anchor,
                          dated(anchor),
                      ),
                  };

        // move a timed series' overrides at their wall times
        if (CalendarEvent.isTimed(event) && CalendarEvent.isTimed(moved)) {
            const zone = event.timeZone ?? timeZone;
            const overrides = Object.keys(event.recurrenceOverrides ?? {}).map((date) => {
                const override = event.recurrenceOverrides?.[date];
                const times =
                    override === undefined || "excluded" in override
                        ? { excluded: true as const }
                        : {
                              start: wallOf(override.start, zone)(minutes),
                              end: wallOf(override.end, zone)(minutes),
                          };

                return [dated(date), times] as const;
            });

            return {
                ...moved,
                ...rules,
                ...(event.recurrenceOverrides === undefined
                    ? {}
                    : { recurrenceOverrides: Object.fromEntries(overrides) }),
            };
        }
        // move a whole-day series' overrides by its days
        else if (!CalendarEvent.isTimed(event) && !CalendarEvent.isTimed(moved)) {
            const overrides = Object.keys(event.recurrenceOverrides ?? {}).map((date) => {
                const override = event.recurrenceOverrides?.[date];
                const times =
                    override === undefined || "excluded" in override
                        ? { excluded: true as const }
                        : { start: dated(override.start), end: dated(override.end) };

                return [dated(date), times] as const;
            });

            return {
                ...moved,
                ...rules,
                ...(event.recurrenceOverrides === undefined
                    ? {}
                    : { recurrenceOverrides: Object.fromEntries(overrides) }),
            };
        }
        // refuse a move that changes the event's kind, which minutes never do
        else {
            throw new TypeError(`calendar series '${event.key}' changed kind as it moved`);
        }
    },

    /** Read an event as a timed one from an instant: at its length, or an hour for whole days. */
    timedAt(event: CalendarEvent, at: number): CalendarEvent {
        const length = CalendarEvent.isTimed(event) ? event.end - event.start : HOUR;

        return { ...fieldsOf(event), start: at, end: at + length };
    },

    /** Read an event as one over whole days from a date, as many days as it covers in a zone. */
    allDayFrom(event: CalendarEvent, date: PlainDate, timeZone: string): CalendarEvent {
        const span = CalendarEvent.span(event, timeZone);
        const days = PlainDate.daysUntil(span.start, span.end);

        return { ...fieldsOf(event), start: date, end: PlainDate.add(date, { days }) };
    },

    /** Give an event a repeat's override of its times, refusing times of the other kind. */
    retime(
        event: CalendarEvent,
        override: { readonly start: PlainDate | number; readonly end: PlainDate | number },
    ): CalendarEvent {
        // take instants for a timed event and dates for one over whole days
        const { start, end } = override;
        if (CalendarEvent.isTimed(event) && typeof start === "number" && typeof end === "number") {
            return { ...event, start, end };
        } else if (
            !CalendarEvent.isTimed(event) &&
            typeof start === "string" &&
            typeof end === "string"
        ) {
            return { ...event, start, end };
        }
        // refuse times of the other kind
        else {
            throw new TypeError(
                `calendar override of '${event.key}' is invalid: its times are another kind`,
            );
        }
    },

    /** Add a moved repeat's times to its repeating event as the override of that repeat. */
    override(event: CalendarEvent, recurrenceId: PlainDate, moved: CalendarEvent): CalendarEvent {
        // override a timed event with instants, and one over whole days with dates
        if (CalendarEvent.isTimed(event) && CalendarEvent.isTimed(moved)) {
            const override = { start: moved.start, end: moved.end };

            return {
                ...event,
                recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: override },
            };
        } else if (!CalendarEvent.isTimed(event) && !CalendarEvent.isTimed(moved)) {
            const override = { start: moved.start, end: moved.end };

            return {
                ...event,
                recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: override },
            };
        }
        // refuse a repeat moved into the other kind
        else {
            throw new TypeError(
                `calendar repeat of '${event.key}' moved into another kind of event`,
            );
        }
    },

    /** Read an event as a draft of a new one that happens once, without its key and repeats. */
    draft(event: CalendarEvent): CalendarEventDraft {
        const {
            key: _key,
            recurrenceRules: _rules,
            recurrenceOverrides: _overrides,
            ...draft
        } = event;

        return draft;
    },

    /** Leave one repeat of an event out, after JSCalendar's excluded overrides. */
    exclude(event: CalendarEvent, recurrenceId: PlainDate): CalendarEvent {
        // exclude by the kind of times the event has, which the override map takes
        const excluded = { excluded: true } as const;
        if (CalendarEvent.isTimed(event)) {
            return {
                ...event,
                recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: excluded },
            };
        } else {
            return {
                ...event,
                recurrenceOverrides: { ...event.recurrenceOverrides, [recurrenceId]: excluded },
            };
        }
    },

    /** Give an event rules, and another event's overrides on the dates a test passes. */
    withRepeats(
        event: CalendarEvent,
        rules: readonly RecurrenceRule[],
        passes: (date: PlainDate) => boolean,
        from: CalendarEvent = event,
    ): CalendarEvent {
        // use instants for a timed event and dates for one over whole days
        if (CalendarEvent.isTimed(event) && CalendarEvent.isTimed(from)) {
            return {
                ...event,
                recurrenceRules: rules,
                recurrenceOverrides: filtered(from.recurrenceOverrides, passes),
            };
        } else if (!CalendarEvent.isTimed(event) && !CalendarEvent.isTimed(from)) {
            return {
                ...event,
                recurrenceRules: rules,
                recurrenceOverrides: filtered(from.recurrenceOverrides, passes),
            };
        }
        // refuse repeats moved onto the other kind
        else {
            throw new TypeError(
                `calendar repeats of '${from.key}' moved onto another kind of event`,
            );
        }
    },
};

/** Select the overrides on the dates a test passes. */
function filtered<Time>(
    overrides: Readonly<Record<PlainDate, RecurrenceOverride<Time>>> | undefined,
    passes: (date: PlainDate) => boolean,
): Record<PlainDate, RecurrenceOverride<Time>> {
    return Object.fromEntries(Object.entries(overrides ?? {}).filter(([date]) => passes(date)));
}

/** Read the fields an event takes into another kind: all but its times and repeats. */
function fieldsOf(
    event: CalendarEvent,
): Omit<CalendarEventFields<never>, "start" | "end" | "recurrenceRules" | "recurrenceOverrides"> {
    return {
        key: event.key,
        title: event.title,
        ...(event.color === undefined ? {} : { color: event.color }),
        ...(event.display === undefined ? {} : { display: event.display }),
        ...(event.source === undefined ? {} : { source: event.source }),
        ...(event.resource === undefined ? {} : { resource: event.resource }),
    };
}

/** Read a series moved to a date as that repeat: keyed by the date, without the series' rules. */
function repeatOf(series: string, moved: CalendarEvent, date: PlainDate): CalendarEvent {
    const { recurrenceRules: _rules, recurrenceOverrides: _overrides, ...single } = moved;

    return { ...single, key: `${series}@${date}` };
}

/** Build the move of an event by wall minutes, its wall times read once for every repeat. */
function shifterOf(event: CalendarEvent, timeZone: string): (minutes: number) => CalendarEvent {
    // move a timed event's ends by the minutes on the wall clock of its zone
    if (CalendarEvent.isTimed(event)) {
        const zone = event.timeZone ?? timeZone;
        const start = wallOf(event.start, zone);
        const end = wallOf(event.end, zone);

        return (minutes) => ({ ...event, start: start(minutes), end: end(minutes) });
    }
    // move a whole-day event's dates, refusing a move within a day
    else {
        return (minutes) => {
            if (minutes % DAY_MINUTES !== 0) {
                throw new RangeError(
                    `calendar event '${event.key}' over whole days moved by '${String(minutes)}' minutes`,
                );
            }
            const days = minutes / DAY_MINUTES;

            return {
                ...event,
                start: PlainDate.add(event.start, { days }),
                end: PlainDate.add(event.end, { days }),
            };
        };
    }
}

/** Read an instant's wall time once, as the instant some wall minutes later. */
function wallOf(at: number, zone: string): (minutes: number) => number {
    // read the wall date and minutes, and the milliseconds past the minute
    const date = TimeZone.date(zone, at);
    const into = PlainTime.minutes(TimeZone.time(zone, at));
    const past = ((at % MINUTE) + MINUTE) % MINUTE;

    return (minutes) => {
        // carry whole days into the date and the rest into the time
        const total = into + minutes;
        const days = Math.floor(total / DAY_MINUTES);
        const time = PlainTime.of(total - days * DAY_MINUTES);

        return TimeZone.instant(zone, PlainDate.add(date, { days }), time) + past;
    };
}

/** Read an instant's place on the wall clock of a zone, in minutes since the epoch's first day. */
function wallMinutesOf(at: number, zone: string): number {
    const days = PlainDate.daysUntil(EPOCH, TimeZone.date(zone, at));

    return days * DAY_MINUTES + PlainTime.minutes(TimeZone.time(zone, at));
}
