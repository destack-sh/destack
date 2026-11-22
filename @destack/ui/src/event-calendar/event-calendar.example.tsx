import { defineExample } from "@destack/package/declare";
import { TimeZone } from "@destack/schema";
import { createSignal, omit, Show } from "@destack/view";
import { Button } from "../button/index.ts";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "../dialog/index.ts";
import { EventCalendar } from "./event-calendar.tsx";
import type {
    CalendarEvent,
    CalendarRepeatMove,
    EventCalendarProperties,
    EventCalendarView,
} from "./control.ts";

/** The time zone of the example's team. */
const ZONE = "Europe/Vienna";

/** Read the instant of a wall time on a day of launch week in Vienna. */
function at(date: string, time: string): number {
    return TimeZone.instant(ZONE, date, time);
}

/** A team's launch week: a conference, launch day, a daily standup and a sync in New York. */
const EVENTS: readonly CalendarEvent[] = [
    {
        key: "conference",
        title: "Web Summit",
        start: "2026-11-18",
        end: "2026-11-20",
        color: "violet",
    },
    {
        key: "launch",
        title: "Launch day",
        start: "2026-11-20",
        end: "2026-11-20",
        color: "orange",
    },
    {
        key: "standup",
        title: "Standup",
        start: at("2026-11-02", "09:00"),
        end: at("2026-11-02", "09:15"),
        recurrenceRules: [{ frequency: "weekly", byDay: [1, 2, 3, 4, 5].map((day) => ({ day })) }],
        recurrenceOverrides: { "2026-11-20": { excluded: true } },
    },
    {
        key: "investors",
        title: "Investor sync",
        start: TimeZone.instant("America/New_York", "2026-11-03", "10:00"),
        end: TimeZone.instant("America/New_York", "2026-11-03", "10:30"),
        timeZone: "America/New_York",
        recurrenceRules: [{ frequency: "weekly" }],
        color: "amber",
    },
    {
        key: "pricing",
        title: "Price the plans",
        start: at("2026-11-16", "09:00"),
        end: at("2026-11-16", "10:30"),
        color: "green",
    },
    {
        key: "post",
        title: "Write the launch post",
        start: at("2026-11-17", "13:00"),
        end: at("2026-11-17", "16:00"),
    },
    {
        key: "demo",
        title: "Record the demo",
        start: at("2026-11-18", "10:00"),
        end: at("2026-11-18", "11:30"),
        color: "tomato",
    },
    {
        key: "retro",
        title: "Launch retro",
        start: at("2026-11-21", "16:00"),
        end: at("2026-11-21", "17:00"),
        color: "teal",
    },
];

/** Render the launch week's calendar in a view, its events moved, resized and created in place. */
function LaunchCalendar(
    properties: Pick<EventCalendarProperties, "dayMaxEvents"> & {
        readonly view: EventCalendarView;
    },
) {
    // keep the events where people leave them, and add the ones they sweep out
    const [events, setEvents] = createSignal<readonly CalendarEvent[]>(EVENTS);
    const save = (...changed: readonly CalendarEvent[]) =>
        setEvents((current) => [
            ...current.map((event) => changed.find((each) => each.key === event.key) ?? event),
            ...changed.filter((each) => !current.some((event) => event.key === each.key)),
        ]);

    // ask what a moved repeat changes, as Google Calendar asks
    const [move, setMove] = createSignal<CalendarRepeatMove | undefined>(undefined);
    const choose = (changed: readonly CalendarEvent[]) => {
        save(...changed);
        setMove(undefined);
    };

    return (
        <>
            <EventCalendar
                {...omit(properties, "view")}
                events={events()}
                defaultDate={"2026-11-18"}
                defaultView={properties.view}
                timeZone={ZONE}
                onEventChange={(changed) => save(changed)}
                onRepeatChange={setMove}
                onCreate={(span) =>
                    save({ key: `new-${String(span.start)}`, title: "New event", ...span })
                }
            />
            <Dialog open={move() !== undefined} onOpenChange={(open) => open || setMove(undefined)}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>Change repeating event</DialogTitle>
                    </DialogHeader>
                    <Show when={move()}>
                        {(chosen) => (
                            <DialogFooter>
                                <Button variant="outline" onClick={() => choose([chosen().only()])}>
                                    This event
                                </Button>
                                <Button
                                    variant="outline"
                                    onClick={() =>
                                        choose(chosen().following(`${chosen().event.key}-next`))
                                    }
                                >
                                    This and following events
                                </Button>
                                <Button onClick={() => choose([chosen().all()])}>All events</Button>
                            </DialogFooter>
                        )}
                    </Show>
                </DialogContent>
            </Dialog>
        </>
    );
}

/** A team's launch week, events moved, resized and created by pointer, touch and keys. */
export const eventCalendarLaunch = defineExample({
    of: EventCalendar,
    name: "launch",
    description:
        "a team's launch week in a month, week, day and agenda view, events moved and created",
    layout: "padded",
    properties: {
        dayMaxEvents: 3,
    },
    render: (properties) => <LaunchCalendar {...properties} view="week" />,
});

/** A team's launch month, each day with its events. */
export const eventCalendarLaunchMonth = defineExample({
    of: EventCalendar,
    name: "launch-month",
    description: "a team's launch month, each day with its events",
    layout: "padded",
    properties: {
        dayMaxEvents: 3,
    },
    render: (properties) => <LaunchCalendar {...properties} view="month" />,
});

/** The open slots a booking page offers in a work week, shaded behind the meetings. */
const SLOTS: readonly CalendarEvent[] = [
    ...EVENTS.filter((event) => event.key !== "retro"),
    ...["2026-11-16", "2026-11-17", "2026-11-19"].map((date): CalendarEvent => ({
        key: `open-${date}`,
        title: "Open for booking",
        start: at(date, "14:00"),
        end: at(date, "16:00"),
        color: "grass",
        display: "background",
    })),
];

/** A work week with the open slots of a booking page shaded behind the meetings. */
export const eventCalendarWorkWeek = defineExample({
    of: EventCalendar,
    name: "work-week",
    description: "a work week with the open slots of a booking page shaded behind the meetings",
    layout: "padded",
    properties: {
        weekends: false,
    },
    render: (properties) => (
        <EventCalendar
            {...properties}
            events={SLOTS}
            defaultDate={"2026-11-18"}
            defaultView="week"
            timeZone={ZONE}
            now={at("2026-11-18", "11:00")}
        />
    ),
});
