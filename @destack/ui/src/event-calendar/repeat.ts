import { PlainDate, Recurrence, type Week } from "@destack/schema";
import { CalendarEvent } from "./event.ts";

/** A changed repeat of a repeating event and what it may change. */
export class CalendarRepeatChange {
    /** The repeating event. */
    readonly event: CalendarEvent;
    /** The date the changed repeat would start on. */
    readonly recurrenceId: PlainDate;
    /** The repeat where it stood. */
    readonly repeat: CalendarEvent;
    /** The repeat where the person left it. */
    readonly moved: CalendarEvent;
    /** Whether it is the series' first repeat, before which none happen. */
    readonly isFirst: boolean;
    /** The time zone the series' wall times repeat in. */
    readonly #timeZone: string;
    /** The weeks the series' rules count in. */
    readonly #week: Week;

    /** Describe a repeat's change from where it stood to where the person left it. */
    constructor(
        event: CalendarEvent,
        recurrenceId: PlainDate,
        repeat: CalendarEvent,
        moved: CalendarEvent,
        timeZone: string,
        week: Week,
    ) {
        // record the change and the zone and weeks it happened in
        this.event = event;
        this.recurrenceId = recurrenceId;
        this.repeat = repeat;
        this.moved = moved;
        this.isFirst = recurrenceId === CalendarEvent.first(event, timeZone);
        this.#timeZone = timeZone;
        this.#week = week;
    }

    /** Change this repeat alone: the event with the repeat's override. */
    only(): CalendarEvent {
        return CalendarEvent.override(this.event, this.recurrenceId, this.moved);
    }

    /** Change every repeat: the series moved as this repeat moved, at the repeat's new length. */
    all(): CalendarEvent {
        // move the series by the wall minutes from the repeat's place by the rules to its new one
        const zone = this.#timeZone;
        const placed = CalendarEvent.repeatOn(this.event, this.recurrenceId, zone);
        const travel = CalendarEvent.travel(placed, this.moved, zone);
        const moved = CalendarEvent.shiftSeries(this.event, travel, zone, this.recurrenceId);
        const timed =
            CalendarEvent.isTimed(moved) && CalendarEvent.isTimed(this.moved)
                ? { ...moved, end: moved.start + this.moved.end - this.moved.start }
                : moved;

        // let this repeat follow the series again, its override dropped at its new date
        const date = CalendarEvent.first(this.moved, zone);

        return CalendarEvent.withRepeats(
            timed,
            timed.recurrenceRules ?? [],
            (each) => each !== date,
        );
    }

    /** Change this repeat and the later ones: the series ended the day before, and a new one. */
    following(key: string): readonly [CalendarEvent, CalendarEvent] {
        // refuse the first repeat, whose following ones are all of them
        if (this.isFirst) {
            throw new TypeError(`calendar repeat '${this.recurrenceId}' is its series' first`);
        }

        // end the series' rules the day before this repeat, with the earlier overrides
        const zone = this.#timeZone;
        const rules = this.event.recurrenceRules ?? [];
        const first = CalendarEvent.first(this.event, zone);
        const { before, after } = Recurrence.split(rules, first, this.recurrenceId, this.#week);
        const ended = CalendarEvent.withRepeats(
            this.event,
            before,
            (date) => date < this.recurrenceId,
        );

        // go on from the moved repeat, the rules left and the later overrides moved as it moved
        const placed = CalendarEvent.repeatOn(this.event, this.recurrenceId, zone);
        const travel = CalendarEvent.travel(placed, this.moved, zone);
        const since = CalendarEvent.first(this.moved, zone);
        const continued = CalendarEvent.withRepeats(
            { ...this.moved, key },
            Recurrence.move(after, this.recurrenceId, since),
            (date) => date > since,
            CalendarEvent.shiftSeries(this.event, travel, zone, this.recurrenceId),
        );

        return [ended, continued];
    }
}

/** A removed repeat of a repeating event and what it may remove. */
export class CalendarRepeatRemoval {
    /** The repeating event. */
    readonly event: CalendarEvent;
    /** The date the removed repeat would start on. */
    readonly recurrenceId: PlainDate;
    /** Whether it is the series' first repeat, before which none happen. */
    readonly isFirst: boolean;
    /** The time zone the series' wall times repeat in. */
    readonly #timeZone: string;
    /** The weeks the series' rules count in. */
    readonly #week: Week;

    /** Describe a repeat the person removed. */
    constructor(event: CalendarEvent, recurrenceId: PlainDate, timeZone: string, week: Week) {
        // record the removal and the zone and weeks it happened in
        this.event = event;
        this.recurrenceId = recurrenceId;
        this.isFirst = recurrenceId === CalendarEvent.first(event, timeZone);
        this.#timeZone = timeZone;
        this.#week = week;
    }

    /** Remove this repeat alone: the event with this repeat left out. */
    only(): CalendarEvent {
        return CalendarEvent.exclude(this.event, this.recurrenceId);
    }

    /** Remove this repeat and the later ones: the event ended the day before. */
    following(): CalendarEvent {
        // refuse the first repeat, whose following ones are all of them
        if (this.isFirst) {
            throw new TypeError(`calendar repeat '${this.recurrenceId}' is its series' first`);
        }

        // end the event's rules the day before this repeat
        const rules = this.event.recurrenceRules ?? [];
        const first = CalendarEvent.first(this.event, this.#timeZone);
        const { before } = Recurrence.split(rules, first, this.recurrenceId, this.#week);

        return CalendarEvent.withRepeats(this.event, before, (date) => date < this.recurrenceId);
    }

    /** Remove every repeat: the repeating event, which the handler removes. */
    all(): CalendarEvent {
        return this.event;
    }
}
