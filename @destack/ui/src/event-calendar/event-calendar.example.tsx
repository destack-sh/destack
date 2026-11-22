import { defineExample } from "@destack/package/declare";
import { TimeZone } from "@destack/schema";
import { createSignal, omit, Show } from "@destack/view";
import { Button } from "../button/index.ts";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "../dialog/index.ts";
import { EventCalendarAgenda } from "./agenda.tsx";
import { EventCalendar, EventCalendarBody, EventCalendarHeader } from "./event-calendar.tsx";
import { EventCalendarMonth } from "./month.tsx";
import { EventCalendarTime } from "./time.tsx";
import { EventCalendarYear } from "./year.tsx";
import { EventCalendarNavigator } from "./navigator.tsx";
import { EventCalendarSources } from "./sources.tsx";
import { EventCalendarPreviewCard } from "./chip.tsx";
import { Label } from "../label/index.ts";
import { Switch } from "../switch/index.ts";
import * as style from "@destack/style";
import type { EventCalendarProperties, EventCalendarView } from "./control.ts";
import type { CalendarEvent } from "./event.ts";
import type { CalendarRepeatChange } from "./repeat.ts";

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
    // save the events where people leave them, and add the ones they sweep out
    const [events, setEvents] = createSignal<readonly CalendarEvent[]>(EVENTS);
    const save = (...changed: readonly CalendarEvent[]) =>
        setEvents((current) => [
            ...current.map((event) => changed.find((each) => each.key === event.key) ?? event),
            ...changed.filter((each) => !current.some((event) => event.key === each.key)),
        ]);

    // ask what a moved repeat changes, as Google Calendar asks
    const [move, setMove] = createSignal<CalendarRepeatChange | undefined>(undefined);
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
            >
                <EventCalendarHeader />
                <EventCalendarBody>
                    <EventCalendarMonth />
                    <EventCalendarTime />
                    <EventCalendarYear />
                    <EventCalendarAgenda />
                    <EventCalendarPreviewCard />
                </EventCalendarBody>
            </EventCalendar>
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
                                <Show when={!chosen().isFirst}>
                                    <Button
                                        variant="outline"
                                        onClick={() =>
                                            choose(chosen().following(`${chosen().event.key}-next`))
                                        }
                                    >
                                        This and following events
                                    </Button>
                                </Show>
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
        >
            <EventCalendarHeader />
            <EventCalendarBody>
                <EventCalendarTime />
                <EventCalendarMonth />
            </EventCalendarBody>
        </EventCalendar>
    ),
});

/** The layout of the team's calendar: a sidebar beside the views. */
const layout = style.create({
    team: {
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        gap: "1.5rem",
        height: "44rem",
    },
    side: {
        display: "flex",
        flexDirection: "column",
        gap: "1rem",
        minWidth: 0,
    },
    main: {
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
        minHeight: 0,
    },
    share: {
        display: "flex",
        alignItems: "center",
        gap: "0.5rem",
    },
});

/** The team's people, whose days the week shows side by side. */
const TEAM: readonly CalendarEvent[] = [
    {
        key: "ada-design",
        title: "Design review",
        source: "work",
        resource: "ada",
        start: at("2026-11-17", "10:00"),
        end: at("2026-11-17", "11:30"),
    },
    {
        key: "grace-interviews",
        title: "Interviews",
        source: "hiring",
        resource: "grace",
        start: at("2026-11-17", "14:00"),
        end: at("2026-11-17", "16:00"),
    },
    {
        key: "ada-dentist",
        title: "Dentist",
        source: "ines",
        resource: "ada",
        start: at("2026-11-18", "08:30"),
        end: at("2026-11-18", "09:30"),
    },
    {
        key: "grace-offsite",
        title: "Offsite",
        source: "work",
        resource: "grace",
        start: "2026-11-19",
        end: "2026-11-20",
    },
];

/** A team's week: people side by side, calendars listed, two zones, and free time shared. */
export const eventCalendarTeam = defineExample({
    of: EventCalendar,
    name: "team",
    description:
        "a team's week with people side by side, calendars listed, two zones and shared free time",
    layout: "padded",
    properties: {},
    render: (properties) => <TeamCalendar {...properties} />,
});

/** Render the team's week beside its navigator and calendar list, free time shared in place. */
function TeamCalendar(properties: Partial<EventCalendarProperties>) {
    // save moves and copies, and mark free time while sharing
    const [events, setEvents] = createSignal<readonly CalendarEvent[]>(TEAM);
    const [isSharing, setSharing] = createSignal(false);
    const hours = [{ start: "09:00", end: "17:00" }] as const;

    return (
        <EventCalendar
            {...properties}
            events={events()}
            defaultDate={"2026-11-16"}
            defaultView="week"
            timeZone={ZONE}
            weekends={false}
            weekNumbers
            workWeek={{ 1: hours, 2: hours, 3: hours, 4: hours, 5: hours }}
            timeZones={[{ zone: "America/New_York", label: "NYC" }]}
            sources={[
                { key: "work", label: "Work", color: "blue" },
                { key: "hiring", label: "Hiring", color: "green" },
                { key: "ines", label: "Ines", display: "busy" },
            ]}
            resources={[
                { key: "ada", label: "Ada" },
                { key: "grace", label: "Grace" },
            ]}
            mode={isSharing() ? "availability" : "events"}
            onEventChange={(changed) =>
                setEvents((current) =>
                    current.map((event) => (event.key === changed.key ? changed : event)),
                )
            }
            onEventAdd={(draft) =>
                setEvents((current) => [
                    ...current,
                    { ...draft, key: `copy-${String(current.length)}` },
                ])
            }
            onEventRemove={(removed) =>
                setEvents((current) => current.filter((event) => event.key !== removed.key))
            }
            xstyle={layout.team}
        >
            <div {...style.attrs(layout.side)}>
                <EventCalendarNavigator />
                <EventCalendarSources />
                <Label xstyle={layout.share}>
                    <Switch checked={isSharing()} onCheckedChange={setSharing} />
                    Share availability
                </Label>
                <Show when={isSharing()}>
                    <span>Sweep free time, then copy it with Mod+Shift+C.</span>
                </Show>
            </div>
            <div {...style.attrs(layout.main)}>
                <EventCalendarHeader />
                <EventCalendarBody>
                    <EventCalendarTime />
                    <EventCalendarMonth />
                    <EventCalendarPreviewCard />
                </EventCalendarBody>
            </div>
        </EventCalendar>
    );
}
