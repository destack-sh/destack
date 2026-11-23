import { t } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createMemo, For, type JSX, Show, useContext, useLocale } from "@destack/view";
import { EventCalendarEventContext, useEventCalendar, type EventCalendarView } from "./control.ts";
import { OfferedView } from "./view.tsx";
import { CalendarEvent } from "./event.ts";
import { EventCalendarEvent } from "./chip.tsx";

/** The width of an agenda day's date, room for a date such as "Wed, 18 Nov". */
const DATE_WIDTH = "8rem";

/** The views the part shows. */
const AGENDA_VIEWS: readonly EventCalendarView[] = ["agenda"];

/** The styles of an agenda. */
const styles = style.create({
    agenda: {
        display: "flex",
        flexDirection: "column",
        margin: 0,
        padding: 0,
        listStyleType: "none",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        overflow: "hidden",
    },
    day: {
        display: "grid",
        gridTemplateColumns: `${DATE_WIDTH} minmax(0, 1fr)`,
        gap: space[3],
        paddingBlock: space[2],
        paddingInline: space[3],
        borderBlockEndWidth: { default: stroke.border, ":last-child": 0 },
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    date: {
        margin: 0,
        fontWeight: weight.medium,
    },
    today: {
        color: swatch.accentText,
    },
    events: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        margin: 0,
        padding: 0,
        listStyleType: "none",
    },
    empty: {
        paddingBlock: space[6],
        textAlign: "center",
        color: color.mutedForeground,
    },
});

/** Render the shown week's days with their events in order. */
export function EventCalendarAgenda(): JSX.Element {
    return <OfferedView views={AGENDA_VIEWS}>{() => <AgendaView />}</OfferedView>;
}

/** Render the agenda view while it shows. */
function AgendaView(): JSX.Element {
    // list each shown day that has events, all-day ones first, then by start
    const control = useEventCalendar();
    const locale = useLocale();
    const Render = useContext(EventCalendarEventContext) ?? EventCalendarEvent;
    const spans = createMemo(() =>
        control
            .events()
            .filter((event) => !CalendarEvent.isBackground(event))
            .toSorted((left, right) => startOf(left) - startOf(right))
            .map((event) => ({ event, span: CalendarEvent.span(event, control.timeZone()) })),
    );
    const days = createMemo(() =>
        control
            .days()
            .map((day) => ({
                day,
                events: spans()
                    .filter(
                        ({ span }) =>
                            PlainDate.compare(span.start, day) <= 0 &&
                            PlainDate.compare(span.end, day) >= 0,
                    )
                    .map(({ event }) => event),
            }))
            .filter((entry) => entry.events.length > 0),
    );

    return (
        <Show
            when={days().length > 0}
            fallback={
                <p data-slot="event-calendar-empty" {...style.attrs(text.sm, styles.empty)}>
                    {locale.render(t`No events this week`)}
                </p>
            }
        >
            <ul data-slot="event-calendar-agenda" {...style.attrs(styles.agenda)}>
                <For each={days()} keyed={(entry) => entry.day}>
                    {(entry) => (
                        <li {...style.attrs(text.sm, styles.day)}>
                            <h3
                                {...style.attrs(
                                    text.sm,
                                    styles.date,
                                    PlainDate.equals(entry().day, control.today()) && styles.today,
                                )}
                            >
                                {locale.plainDate(entry().day, {
                                    weekday: "short",
                                    day: "numeric",
                                    month: "short",
                                })}
                            </h3>
                            <ul {...style.attrs(styles.events)}>
                                <For each={entry().events}>
                                    {(event) => (
                                        <li>
                                            <Render event={event} layout="list" isStart isEnd />
                                        </li>
                                    )}
                                </For>
                            </ul>
                        </li>
                    )}
                </For>
            </ul>
        </Show>
    );
}

/** Read an event's order in a day: all-day events first, then timed ones by start. */
function startOf(event: CalendarEvent): number {
    return CalendarEvent.isTimed(event) ? event.start : Number.NEGATIVE_INFINITY;
}
